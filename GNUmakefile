.PHONY: all clean run run-network run-network-client iso

KERNEL := target/x86_64-unknown-none/release/kernel
ISO := image.iso
LIMINE_DIR := limine

all: $(ISO)

$(KERNEL): kernel/src/**/*.rs renderer/src/**/*.rs protocol/src/**/*.rs Cargo.toml
	cargo build --release

$(LIMINE_DIR):
	git clone https://github.com/limine-bootloader/limine.git --branch=v8.x-binary --depth=1
	$(MAKE) -C $(LIMINE_DIR)

$(ISO): $(KERNEL) $(LIMINE_DIR) limine.conf
	mkdir -p iso_root/boot/limine iso_root/EFI/BOOT
	cp $(KERNEL) iso_root/kernel
	cp limine.conf iso_root/boot/limine/
	cp $(LIMINE_DIR)/limine-bios.sys $(LIMINE_DIR)/limine-bios-cd.bin iso_root/boot/limine/
	cp $(LIMINE_DIR)/BOOTX64.EFI iso_root/EFI/BOOT/
	cp $(LIMINE_DIR)/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot EFI/BOOT/BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(ISO)
	$(LIMINE_DIR)/limine bios-install $(ISO)

run: $(ISO)
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0 \
		-netdev user,id=net0,hostfwd=udp::5000-:5000 \
		-no-reboot \
		-d int,cpu_reset -D qemu.log

run-network: $(ISO)
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0,mac=52:54:00:12:34:56 \
		-netdev socket,id=net0,listen=:1234 \
		-no-reboot

run-network-client: $(ISO)
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0,mac=52:54:00:12:34:57 \
		-netdev socket,id=net0,connect=127.0.0.1:1234 \
		-no-reboot

clean:
	cargo clean
	rm -rf iso_root $(ISO)
