.PHONY: all clean run run-software run-benchmark run-network run-network-client iso

KERNEL := target/x86_64-unknown-none/release/kernel
ISO := image.iso
LIMINE_DIR := limine

# QEMU audio device (Intel HDA for broad compatibility)
# Use coreaudio on macOS, sdl or pulseaudio on Linux
QEMU_AUDIO = -audiodev coreaudio,id=audio0 -device intel-hda -device hda-duplex,audiodev=audio0

# USB tablet for absolute mouse positioning
QEMU_MOUSE = -device usb-ehci -device usb-tablet

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

# Single instance (for standalone testing)
run-single: $(ISO)
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0 \
		-netdev user,id=net0,hostfwd=udp::5000-:5000 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot \
		-d int,cpu_reset -D qemu.log

# Combined server + client launch (default run target)
# Server runs headless (logs only), client has GUI
run: $(ISO)
	@echo "Starting BattleRoyaleOS Server (headless)..."
	@qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-nographic \
		-cdrom $(ISO) \
		-device e1000,netdev=net0,mac=52:54:00:12:34:56 \
		-netdev socket,id=net0,listen=:1234 \
		-no-reboot &
	@sleep 2
	@echo "Starting BattleRoyaleOS Client (GUI)..."
	@qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom $(ISO) \
		-serial null \
		-device e1000,netdev=net0,mac=52:54:00:12:34:57 \
		-netdev socket,id=net0,connect=127.0.0.1:1234 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot

# Run server only (headless, logs to terminal)
run-server: $(ISO)
	@echo "Starting BattleRoyaleOS Server (headless)..."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-nographic \
		-cdrom $(ISO) \
		-device e1000,netdev=net0,mac=52:54:00:12:34:56 \
		-netdev socket,id=net0,listen=:1234 \
		-no-reboot

# Run client only (GUI, connects to server)
run-client: $(ISO)
	@echo "Starting BattleRoyaleOS Client (GUI)..."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0,mac=52:54:00:12:34:57 \
		-netdev socket,id=net0,connect=127.0.0.1:1234 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot

# Stop all running instances
stop:
	@pkill -f "qemu-system-x86_64.*$(ISO)" || true
	@echo "Stopped all BattleRoyaleOS instances"

run-software: $(ISO)
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0 \
		-netdev user,id=net0,hostfwd=udp::5000-:5000 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot \
		-d int,cpu_reset -D qemu.log

# Benchmark mode - auto-starts InGame for performance testing
benchmark-iso: $(KERNEL) $(LIMINE_DIR)
	mkdir -p iso_root/boot/limine iso_root/EFI/BOOT
	cp $(KERNEL) iso_root/kernel
	cp limine-benchmark.conf iso_root/boot/limine/limine.conf
	cp $(LIMINE_DIR)/limine-bios.sys $(LIMINE_DIR)/limine-bios-cd.bin iso_root/boot/limine/
	cp $(LIMINE_DIR)/BOOTX64.EFI iso_root/EFI/BOOT/
	cp $(LIMINE_DIR)/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot EFI/BOOT/BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o benchmark.iso
	$(LIMINE_DIR)/limine bios-install benchmark.iso

run-benchmark: benchmark-iso
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom benchmark.iso \
		-serial stdio \
		-device e1000,netdev=net0 \
		-netdev user,id=net0,hostfwd=udp::5000-:5000 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot \
		-d int,cpu_reset -D qemu.log

run-network: $(ISO)
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0,mac=52:54:00:12:34:56 \
		-netdev socket,id=net0,listen=:1234 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot

run-network-client: $(ISO)
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom $(ISO) \
		-serial stdio \
		-device e1000,netdev=net0,mac=52:54:00:12:34:57 \
		-netdev socket,id=net0,connect=127.0.0.1:1234 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot

clean:
	cargo clean
	rm -rf iso_root $(ISO)
