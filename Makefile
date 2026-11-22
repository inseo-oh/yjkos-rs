TARGET_DIR        := target
SUPPORT_DIR       := support
RELEASE           ?= 0

ifeq ($(RELEASE),1)
OUTPUT_TYPE := release
else
OUTPUT_TYPE := debug
endif

BOOTROOT := $(TARGET_DIR)/$(OUTPUT_TYPE)_bootroot

TARGET_YJKOS    := $(TARGET_DIR)/i586-yjkos/$(OUTPUT_TYPE)/yjkos
TARGET_YJKERNEL := $(TARGET_DIR)/i586-yjkernel/$(OUTPUT_TYPE)/yjkernel
TARGET_ISO      := yjkos_i586_$(OUTPUT_TYPE).iso


GRUB_MKRESCUE := grub-mkrescue
GDB         := gdb
CP          := cp
CARGO       := cargo
CARGO_BUILD := $(CARGO) +nightly build
ifeq ($(RELEASE),1)
CARGO_BUILD += --release
endif

KERNEL_JSON_TARGET := $(SUPPORT_DIR)/build/i586-yjkernel.json
USER_JSON_TARGET   := $(SUPPORT_DIR)/build/i586-yjkos.json

all: $(TARGET_ISO)

$(TARGET_ISO): $(TARGET_YJKERNEL) $(TARGET_YJKOS)
	rm -rf $(BOOTROOT)
	mkdir -p $(BOOTROOT)
	mkdir -p $(BOOTROOT)/boot/grub/
	$(CP) $(SUPPORT_DIR)/build/grub.cfg $(BOOTROOT)/boot/grub/
	$(CP) $(TARGET_YJKERNEL) $(BOOTROOT)/boot/
	$(CP) $(TARGET_YJKOS) $(BOOTROOT)/boot/
	$(GRUB_MKRESCUE) -o $@ $(BOOTROOT)
	rm -rf $(BOOTROOT)

$(TARGET_YJKERNEL): build-yjkernel
$(TARGET_YJKOS):    build-yjkos

build-yjkernel: $(TARGET_YJKOS)
	$(CP) $(TARGET_YJKOS) $(TARGET_DIR)/latest-yjkos
	$(CARGO_BUILD) -p yjkernel -Z build-std=core,alloc -Zjson-target-spec --target $(KERNEL_JSON_TARGET)

build-yjkos:
	$(CARGO_BUILD) -p yjkos -Z build-std=core,alloc -Zjson-target-spec --target $(USER_JSON_TARGET)

run: $(TARGET_ISO)
	qemu-system-i386 -d guest_errors$(QEMU_DEBUG_FLAGS) -no-reboot -s -m 64M -nographic -serial mon:stdio -net none -boot d \
		-drive id=cd0,file=$<,format=raw,if=none -device ide-cd,drive=cd0 \
		
gdb: $(TARGET_YJKERNEL)
	gdb -ex "set architecture i386" \
	    -ex "target remote :1234" \
	    $(TARGET_YJKERNEL)

clean:
	cargo clean
	rm -f yjkos_i586_debug.iso yjkos_i586_release.iso

.PHONY: all run gdb clean build-yjkernel build-yjkos
