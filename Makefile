all:
	cd ./kernel && make testcases-img && make build
	mv os.bin kernel-qemu
	cp opensbi-qemu.bin sbi-qemu
