# Профіль фізичного заліза owner

Source repository: juv4uk/wsm-os
Source file: docs/OWNER-HARDWARE-PROFILE.md
Source SHA: cb0fa1af003b3a0a6ed61a94ce3ebb065a0fe32b
Snapshot captured: 2026-09-22

Це локальний reference snapshot фізичної машини. Він потрібен, щоб реконструкція прямо називала своє реальне залізо, а не абстрактний x86-64.

## Фізична платформа

Motherboard: Gigabyte Technology Co., Ltd. H170-Gaming 3
BIOS: American Megatrends F22e
BIOS date: 2018-03-09
Firmware: UEFI
Host OS: Windows 11 Pro for Workstations, 64-bit, build 26200

## CPU

Model: Intel Core i5-6400 @ 2.70 GHz
Microarchitecture: Skylake
Family/model/stepping: family 6, model 94, stepping 3
Topology: 1 socket, 4 cores, 4 logical CPUs, SMT not shown
Endianness: little-endian
Visible physical address width: 39-bit
Visible virtual address width: 48-bit
L1d: 4 × 32 KiB
L1i: 4 × 32 KiB
L2: 4 × 256 KiB
L3: 6 MiB shared

## ISA / CPU capability facts

Baseline:
X86-BASE, X86-64, X87, MMX, SSE, SSE2, SSE3, SSSE3, SSE4.1, SSE4.2

Runtime-gated:
AES-NI, PCLMULQDQ, AVX, F16C, FMA3, BMI1, BMI2, AVX2, RDRAND, RDSEED, ADX, XSAVE, CLFLUSHOPT

Platform-gated:
MPX, SGX

Explicitly unavailable:
TSX, AVX-512, AMX

AVX and AVX2 require the runtime CPUID/OSXSAVE/XGETBV conditions documented by the upstream profile.

## Memory

Physical host: 16 GiB.
DIMM 1: 8 GiB DDR4-2133, CT8G4DFS8213.C8FBD1.
DIMM 2: 8 GiB DDR4-2133, TEAMGROUP-UD4-2133.
Current WSL allocation: 7.7 GiB RAM.
WSL swap: 2.0 GiB.

## GPU

Intel HD Graphics 530.
NVIDIA GeForce GTX 1050 Ti, 4096 MiB, CUDA capability 6.1 / sm_61.

GPU is not a prerequisite for the historical evaluator. The reconstruction target is CPU/x86-64.

## Storage

Kingston SNV2S1000G — NVMe SSD, 1 TB.
Samsung SSD 850 EVO — SATA SSD, 120 GB.
Samsung HD321KJ — SATA HDD, 320 GB.

mccarthy-eval must never select, partition, format or write a physical disk automatically.

## Network

Killer E2200 Gigabit Ethernet.
Tailscale virtual adapter.

Network is outside the historical evaluator core.

## Development environment

WSL2 kernel: 6.18.33.2-microsoft-standard-WSL2
Distribution: Ubuntu 24.04.4 LTS
WSL CPUs: 4 online
Rust: 1.98.0
Installed Rust targets: x86_64-unknown-linux-gnu, x86_64-pc-windows-gnu, wasm32-unknown-unknown
Guix: available
Bare-metal tooling and qemu-system-x86_64 are available through the current Guix setup.

## Reconstruction target

Primary correctness target:

Intel Core i5-6400 / Skylake / x86-64 / 64-bit / little-endian.

Correctness path should prefer scalar x86-64 and must not require AVX2/BMI2 or other optional extensions.
