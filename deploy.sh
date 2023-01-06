#! /bin/bash

cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --out-dir ./out/ --target web ./target/wasm32-unknown-unknown/release/ld52.wasm
cp -r assets/ out/
zip -r out.zip out
butler push out.zip zjikra/ld52:wasm
