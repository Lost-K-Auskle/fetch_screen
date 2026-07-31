"""Extract as.exe from MSYS2 binutils package"""
import zstandard as zstd
import tarfile
import os
import shutil

zst_path = os.path.expanduser("~/Downloads/mingw-binutils.tar.zst")
extract_dir = "/tmp/mingw-binutils"
os.makedirs(extract_dir, exist_ok=True)

with open(zst_path, "rb") as f:
    dctx = zstd.ZstdDecompressor()
    with dctx.stream_reader(f) as reader:
        with tarfile.open(fileobj=reader, mode="r|") as tar:
            found = False
            for member in tar:
                if member.name.endswith("/as.exe"):
                    print(f"Found: {member.name}")
                    tar.extract(member, extract_dir)
                    found = True
                    break

            if not found:
                print("as.exe not found. Listing all members:")
                tar.extractall(extract_dir)
                for root, dirs, files in os.walk(extract_dir):
                    for f in files:
                        if "as" in f.lower() or "bin" in root.lower():
                            print(f"  {os.path.join(root, f)}")
                print("Done listing")
                exit(1)

# Copy as.exe to toolchain
for root, dirs, files in os.walk(extract_dir):
    for file in files:
        if file == "as.exe":
            src = os.path.join(root, file)
            dest = r"C:\Users\lost\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained\as.exe"
            shutil.copy2(src, dest)
            print(f"Copied as.exe -> {dest}")
            print(f"Size: {os.path.getsize(dest)} bytes")
            exit(0)

print("as.exe not extracted")
exit(1)
