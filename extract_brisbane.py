import base64
import io
import json
import zlib

import numpy as np


def extract_quantum_data():
    print("Loading IBM Brisbane results...")
    with open("job-d29pavbp64qc73ein7j0-result.json", "r") as f:
        data = json.load(f)

    # Navigate the Qiskit v2 PrimitiveResult JSON structure
    b64_str = data["__value__"]["pub_results"][0]["__value__"]["data"]["__value__"][
        "fields"
    ]["meas"]["__value__"]["array"]["__value__"]

    # Decode base64 -> Decompress Zlib -> Load Numpy array
    print("Decoding compressed payload...")
    compressed_data = base64.b64decode(b64_str)
    npy_data = zlib.decompress(compressed_data)

    # Read the measurement array
    arr = np.load(io.BytesIO(npy_data))

    # Flatten the multidimensional array and pack it into raw bytes
    flat_bits = arr.flatten()
    packed_bytes = np.packbits(flat_bits)

    # Save to a flat binary file for Rust to ingest
    out_file = "brisbane_raw.bin"
    packed_bytes.tofile(out_file)

    print(
        f"Extraction complete! Saved {len(packed_bytes)} bytes of pure quantum data to {out_file}."
    )
    print(f"Total available quantum bits: {len(flat_bits)}")


if __name__ == "__main__":
    extract_quantum_data()
