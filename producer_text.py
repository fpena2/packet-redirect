import socket
import time

def send_udp_packets():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    target_address = ('localhost', 3000)

    i = 0
    try:
        while True:
            message = f"Hello World {i}".encode('utf-8')
            sock.sendto(message, target_address)
            print(f"Sent: {message.decode('utf-8')}")
            time.sleep(1)
            i += 1
    except KeyboardInterrupt:
        print("\nStopped sending packets.")
    finally:
        sock.close()

if __name__ == "__main__":
    send_udp_packets()
