#       Fireblocks Account Setup and Configuration
###     Authenticator App Setup

- Download and install an authenticator app such as Google Authenticator, Authy, or Microsoft Authenticator on your mobile device from the Apple App Store or Google Play Store.
- Accept the invite from Fireblocks to join the workspace.
- Set up your Fireblocks account by using the authenticator app on your mobile device to scan the QR code displayed on the Fireblocks interface. This will link your Fireblocks account with the authenticator app. Once the QR code is scanned, the authenticator app will generate a verification code. Enter this code in the appropriate field on the Fireblocks interface to verify the setup.

###     Fireblocks App Installation

 - Install the Fireblocks app on your mobile device from the Apple App Store or Google Play Store.
 - Scan the QR code from the Fireblocks app and create a recovery code to setup your signing key. 
 - The MPC setup will be complete after owner approval

###     API User Setup

1. Generate a Certificate Signing Request (CSR) file required for authenticating the API user with Fireblocks. To do this, open a command line interface and run the following command:

```sh
    openssl req -new -newkey rsa:4096 -nodes -keyout fireblocks_secret.key -out fireblocks.csr
```
This command will generate a new RSA key pair with a key length of 4096 bits, save the private key to a file named "fireblocks_secret.key", and create a CSR file named "fireblocks.csr". Make sure to keep the Fireblocks API secret key (fireblocks_secret.key) safe and secure, and avoid sharing it with anyone else.

2. Click "Add User" under the Users tab in settings, choose "API USER", and enter a name for the user.
3. Choose the appropriate Role for the API USER. Since the user will only be used for signing transactions, select the "signer" role.
4. Upload the CSR file generated earlier using the "Choose File" option.

![image](https://user-images.githubusercontent.com/45506001/230668903-ba7ecb9d-3f85-48d3-a109-f316fa8f579e.png)

6. Submit the request for owner approval. Once approved, the API USER will be created.
7. Copy the API Key of the newly created Api user.
8. Set the environment variables **FIREBLOCKS_API_KEY** and **SECRET_PATH** in the hub-treasuries by providing the API key value for FIREBLOCKS_API_KEY and the file path for fireblocks_secret.key as the value for SECRET_PATH.


### Create Transaction Authorization Policy

To create drop and mint an edition using Fireblocks API, the API user must be granted  permission to make raw transaction calls.The raw message needs to be signed by the api user and submitted to the RPC.The TAP should be configured to allow the **Raw** transaction type with the signer set as Initiator.

Similarly, for transferring funds from a vault account to another wallet, the TAP should allow **Transfer** transaction type for the API user, granting them the necessary authorization for wallet-to-wallet transfers.

![image](https://user-images.githubusercontent.com/45506001/230668764-04e02bf1-82d7-4bb2-8a86-91f1bb787fe8.png)

https://support.fireblocks.io/hc/en-us/articles/4407708176156-Creating-rules-using-the-TAP-Editor