# The BitcoinPhone

Voice over the Bitcoin protocol.


### Setup Instructions for Mac
1. Install rust https://www.rust-lang.org/tools/install
2. Install homebrew https://brew.sh/
3. Run `brew install pkg-config` and `brew install portaudio`

### Setup Instructions for Linux
1. Install rust https://www.rust-lang.org/tools/install

### Setup Instructions for Windows
Wasn't able to get this working, sorry :(



### Steps to run
Note: Have your partner repeat these instructions on their computer too.
1. Open a terminal 
2. Run `git clone https://github.com/gamebyte-dev/bitcoinphone`
3. Run `cd ./bitcoinphone/bitcoinphone`
4. Run `cargo run` (this should be successful)
5. Close the program (Ctrl+C)
6. Run `cargo run` again (sorry this is a bug still not fixed)
7. Send at least 23000 satoshis to the funding address on the screen
8. Close the program (Ctrl+C) 
9. Run `cargo run` (this should be successful)
10. Paste in your partners communication address this is not the same as the funding address.
11. You should see some synchronization text but voila. Voila voice over bitcoin!