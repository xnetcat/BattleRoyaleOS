.PHONY: all clean run run-single run-server run-client run-benchmark run-test run-network run-network-client iso stop

KERNEL := target/x86_64-unknown-none/release/kernel
ISO := image.iso
SERVER_ISO := server.iso
BENCHMARK_ISO := benchmark.iso
TEST_ISO := test.iso
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

# Main ISO with boot menu (Game, Server, Benchmark, Test)
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

# Server ISO (auto-boots to server mode, no menu)
$(SERVER_ISO): $(KERNEL) $(LIMINE_DIR) limine-server.conf
	mkdir -p iso_root/boot/limine iso_root/EFI/BOOT
	cp $(KERNEL) iso_root/kernel
	cp limine-server.conf iso_root/boot/limine/limine.conf
	cp $(LIMINE_DIR)/limine-bios.sys $(LIMINE_DIR)/limine-bios-cd.bin iso_root/boot/limine/
	cp $(LIMINE_DIR)/BOOTX64.EFI iso_root/EFI/BOOT/
	cp $(LIMINE_DIR)/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot EFI/BOOT/BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(SERVER_ISO)
	$(LIMINE_DIR)/limine bios-install $(SERVER_ISO)

# Benchmark ISO (auto-boots to benchmark mode, no menu)
$(BENCHMARK_ISO): $(KERNEL) $(LIMINE_DIR) limine-benchmark.conf
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
		iso_root -o $(BENCHMARK_ISO)
	$(LIMINE_DIR)/limine bios-install $(BENCHMARK_ISO)

# Test ISO (auto-boots to test mode with all items spawned, no menu)
$(TEST_ISO): $(KERNEL) $(LIMINE_DIR) limine-test.conf
	mkdir -p iso_root/boot/limine iso_root/EFI/BOOT
	cp $(KERNEL) iso_root/kernel
	cp limine-test.conf iso_root/boot/limine/limine.conf
	cp $(LIMINE_DIR)/limine-bios.sys $(LIMINE_DIR)/limine-bios-cd.bin iso_root/boot/limine/
	cp $(LIMINE_DIR)/BOOTX64.EFI iso_root/EFI/BOOT/
	cp $(LIMINE_DIR)/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot EFI/BOOT/BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(TEST_ISO)
	$(LIMINE_DIR)/limine bios-install $(TEST_ISO)

# Single instance with boot menu (for standalone testing)
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
# Server runs headless (logs only), client has GUI with boot menu
run: $(SERVER_ISO) $(ISO)
	@echo "Starting BattleRoyaleOS Server (headless)..."
	@qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-nographic \
		-cdrom $(SERVER_ISO) \
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

# Run server only (headless, logs to terminal, auto-boots server mode)
run-server: $(SERVER_ISO)
	@echo "Starting BattleRoyaleOS Dedicated Server..."
	@echo "Server will run headless with logs to terminal."
	@echo "Press Ctrl+C to stop."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-nographic \
		-cdrom $(SERVER_ISO) \
		-device e1000,netdev=net0,mac=52:54:00:12:34:56 \
		-netdev socket,id=net0,listen=:1234 \
		-no-reboot

# Run client only (GUI with boot menu, connects to server)
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
	@pkill -f "qemu-system-x86_64.*\.iso" || true
	@echo "Stopped all BattleRoyaleOS instances"

# Benchmark mode - auto-starts InGame for performance testing
run-benchmark: $(BENCHMARK_ISO)
	@echo "Starting BattleRoyaleOS Benchmark Mode..."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom $(BENCHMARK_ISO) \
		-serial stdio \
		-device e1000,netdev=net0 \
		-netdev user,id=net0,hostfwd=udp::5000-:5000 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot \
		-d int,cpu_reset -D qemu.log

# Test mode - spawns all items for testing functionality
run-test: $(TEST_ISO)
	@echo "Starting BattleRoyaleOS Test Mode..."
	@echo "All weapons, chests, and items will spawn around player."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-smp 5 \
		-vga vmware \
		-cdrom $(TEST_ISO) \
		-serial stdio \
		-device e1000,netdev=net0 \
		-netdev user,id=net0,hostfwd=udp::5000-:5000 \
		$(QEMU_AUDIO) \
		$(QEMU_MOUSE) \
		-no-reboot \
		-d int,cpu_reset -D qemu.log

# Legacy targets for compatibility
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
	rm -rf iso_root $(ISO) $(SERVER_ISO) $(BENCHMARK_ISO) $(TEST_ISO)
