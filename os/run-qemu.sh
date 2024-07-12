#cargo clean
make build
rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/os -O binary target/riscv64gc-unknown-none-elf/release/os.bin
qemu-system-riscv64 -machine virt -m 256M -nographic -bios ./../bootloader/rustsbi-qemu.bin -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000