# QUICKLY RUN THE PROGRAM FOR TESTING:

# Terminal A

cargo run -p node -- --log info serve

# Terminal B

echo "framed and flawless" > /tmp/ro_msg.txt && \
HASH=$(cargo run -p node -- put /tmp/ro_msg.txt | tail -n1) && \
cargo run -p node -- get "$HASH" /tmp/ro_msg_out.txt && \
diff -u /tmp/ro_msg.txt /tmp/ro_msg_out.txt && echo "PUT/GET OK ✅"