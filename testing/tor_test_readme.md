## Use a split terminal  

# 1st Terminal:

STAY_UP=1 TOR_BOOTSTRAP_TIMEOUT=600 bash testing/test_tor.sh

# 2nd Terminal:

printf 'AUTHENTICATE "rotest"\r\nADD_ONION NEW:ED25519-V3 Flags=DiscardPK Port=1777,127.0.0.1:1777\r\n' \ | nc -w 3 127.0.0.1 19069

## NEW

TOR_BOOTSTRAP_TIMEOUT=600 ONION_WAIT_TIMEOUT=300 \
DO_E2E=1 STAY_UP=1 RUST_LOG=info \
bash testing/test_tor.sh


## Zip logs

TOR_BOOTSTRAP_TIMEOUT=600 DO_E2E=1 STAY_UP=1 RUST_LOG=info \
bash testing/test_tor.sh |& tee >(gzip -9c > /tmp/tor_test_console.$(date +%F_%H%M%S).log.gz)


## NEWWWW

export DO_E2E=0
export SOCKS_PORT=9050
export CTRL_PORT=9051
bash ./testing/test_tor.sh


## NEWWWWWWWWWW
SOCKS_PORT=19100 CTRL_PORT=19101 bash ./testing/test_tor.sh
