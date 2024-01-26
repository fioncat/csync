# Csync Security

To enhance your privacy and security, csync supports encrypting clipboard data before transmitting it over the network and automatically decrypting data received from other devices. We employ the secure AES-256-GCM algorithm for encryption.

To enable encrypted transmission, you need to provide a password. This password will be used to generate the AES-256-GCM key. The key generation utilizes the PBKDF2-SHA256 algorithm and includes salting to ensure the password cannot be easily cracked.