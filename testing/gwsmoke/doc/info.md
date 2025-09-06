Enter into terminal:

cargo run -p gwsmoke -- \
  --build \
  --root . \
  --out-dir .onions \
  --bind 127.0.0.1:0 \
  --http-wait-sec 25 \
  --stream \
  --rust-log trace
