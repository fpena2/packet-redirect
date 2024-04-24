import socket
import time
from io import BytesIO
import secrets
import string

from opendis.DataOutputStream import DataOutputStream
from opendis.dis7 import EntityStatePdu
from opendis.RangeCoordinates import *

UDP_PORT = 9000
DESTINATION_ADDRESS = "0.0.0.0"

udpSocket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
udpSocket.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)

gps = GPS() # conversion helper

def send():
    pdu = EntityStatePdu()
    pdu.entityID.entityID = 42
    pdu.entityID.siteID = 17
    pdu.entityID.applicationID = 23
    marking_string = ''.join(secrets.choice(string.ascii_letters + string.digits) for _ in range(5))
    pdu.marking.setString(marking_string)

     # Entity in Monterey, CA, USA facing North, no roll or pitch
    montereyLocation = gps.llarpy2ecef(deg2rad(36.6),   # longitude (radians)
                                       deg2rad(-121.9), # latitude (radians)
                                       1,               # altitude (meters)
                                       0,               # roll (radians)
                                       0,               # pitch (radians)
                                       0                # yaw (radians)
                                       )

    pdu.entityLocation.x = montereyLocation[0]
    pdu.entityLocation.y = montereyLocation[1]
    pdu.entityLocation.z = montereyLocation[2]
    pdu.entityOrientation.psi = montereyLocation[3]
    pdu.entityOrientation.theta = montereyLocation[4]
    pdu.entityOrientation.phi = montereyLocation[5]


    memoryStream = BytesIO()
    outputStream = DataOutputStream(memoryStream)
    pdu.serialize(outputStream)
    data = memoryStream.getvalue()

    udpSocket.sendto(data, (DESTINATION_ADDRESS, UDP_PORT))
    print("Sent {} for {}. {} bytes".format(pdu.__class__.__name__, pdu.marking.charactersString(), len(data)))
    time.sleep(60)

while True:
    send()
