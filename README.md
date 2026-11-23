# Press Play ATLAS — Control Center (Rust)

![Rust](https://img.shields.io/badge/Rust-Edition%202024-orange?logo=rust)
![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6?logo=windows)
![Sensor](https://img.shields.io/badge/Sensor-PAW3311%20(50--24000%20DPI)-00D6FF)
![Build](https://img.shields.io/badge/Binary-~6%20MB%20Single%20EXE-2EE08A)

Aplikasi kontrol **native Rust** yang super cepat, ringan, dan modern untuk mouse gaming **Press Play ATLAS** — dibuat sebagai pengganti software bawaan (`Mouse.exe`) yang lambat dibuka dan memiliki tampilan jadul.

> **Dibuat oleh Eyjel** · Ditulis 100% dalam Rust dengan komunikasi **Win32 USB HID langsung** tanpa Electron, tanpa WebView, dan tanpa DLL eksternal.

---

## Kenapa Dibuat Ulang dengan Rust?

| Aspek | Software Bawaan (`Mouse.exe`) | Press Play ATLAS Control Center (Rust) |
| :--- | :--- | :--- |
| **Kecepatan Buka** | Lambat (DuiLib + aset gambar besar) | **Instan** (GPU-accelerated `eframe`/`egui`) |
| **Sinkronisasi Hardware** | Ada `Sleep(500ms–1000ms)` yang bikin freeze | **Non-blocking background HID thread** |
| **Tampilan UI** | Jadul, statis, banyak menu tidak relevan (RGB palsu) | **Modern, stealth, interaktif, tanpa fitur RGB palsu** |
| **Distribusi** | Banyak DLL (`DuiLib`, `hidapi`, `hiddriver_1/2`, dll.) | **1 file `PressPlayATLAS.exe` mandiri (~6 MB)** |
| **Penyimpanan Profil** | File biner `pro.data` tertutup | **JSON rapi** (mudah dibackup, impor, dan ekspor) |

---

## Fitur Utama

### 1. Peta Tombol Interaktif (*Tombol*)
- Visualisasi vektor bodi ergonomis **Press Play ATLAS** dengan pencahayaan studio yang mengikuti arah kursor.
- **Tes Hardware Langsung**: Tekan tombol fisik mouse (`LMB`, `RMB`, `MMB`, `M4`, `M5`) atau putar scroll wheel untuk melihat animasi tekanan, gelombang *ripple*, dan putaran scroll secara real-time di layar.
- Remapping lengkap untuk 6 tombol (termasuk tombol DPI bawah): fungsi Mouse, DPI, Media, Browser, Pintasan Windows, Kombinasi Keyboard Kustom, hingga Makro.
- Dilengkapi pengaman agar minimal satu tombol tetap berfungsi sebagai **Klik Kiri**.

### 2. Sensor & DPI (*PAW3311 Engine*)
- **6 hingga 8 Level DPI** (`50 – 24.000 DPI`) dengan *equalizer bar* yang bisa langsung diseret naik/turun serta preset cepat (`400 / 800 / 1600 / 3200`).
- **Polling Rate**: Pilihan `125 Hz`, `250 Hz`, `500 Hz`, dan `1000 Hz` lengkap dengan latensi (`8.0 ms` – `1.0 ms`).
- **Pad Tes Sensor & Hz Langsung**: Area interaktif untuk mengukur *Live Hz*, *Peak Hz*, grafik *sparkline*, dan jejak gerakan (*motion trail*).
- **Kalibrasi Presisi**:
  - **Lift-Off Distance (LOD)**: `1.0 mm` / `2.0 mm` dengan diagram ketinggian sensor optik beranimasi.
  - **Respon Klik (Key Debounce)**: `2 ms – 50 ms` (step `2 ms`).
  - **Ripple Control** & **Angle Snapping**.
  - Pintasan cepat ke *Windows Mouse Properties* (`main.cpl`).

### 3. Studio Makro + All Drag Delay (*Makro*)
- Rekam tombol keyboard secara langsung (*live recording*) atau tambahkan aksi keyboard/mouse secara manual.
- **All Drag Delay**: Geser **1 slider** untuk mengubah/menurunkan delay `ms` **seluruh langkah makro sekaligus** (plus tombol cepat `-20ms`, `-10ms`, `-5ms`, `+5ms`, `÷2`, dan `Min 10ms`) dengan indikator total durasi real-time.
- Mendukung 4 mode pengulangan (ulangi N kali, berhenti saat tekan tombol lain, tahan tombol, atau toggle) dan pasang cepat 1-klik ke tombol `M4` / `M5`.

### 4. Daya, Profil & Diagnostik USB HID (*Daya & Profil*)
- **Gauge Baterai Cincin Animasi** dengan status *charging* dan estimasi sisa jam pakai.
- Pengaturan **Waktu Tidur Ringan (Sleep)** & **Waktu Tidur Dalam (Deep Sleep)** (`1 – 60 menit`).
- **Multi-Profil**: Buat, duplikat, ganti nama, hapus, reset bawaan pabrik, serta **Impor/Ekspor JSON**.
- **5 Pilihan Warna Aksen UI** (*Arctic Cyan*, *ATLAS Crimson*, *Volt Mint*, *Titanium Gold*, *Stealth Silver*).
- **Log Komunikasi USB HID Real-Time** untuk memantau paket TX/RX ke mouse.

---

## Dukungan Hardware & Protokol HID

Aplikasi ini berkomunikasi langsung dengan firmware Press Play ATLAS melalui Windows HID API (`hid.dll` + `setupapi.dll` + `kernel32.dll`):

| Mode Koneksi | Vendor ID (VID) | Product ID (PID) | Endpoint Feature | Endpoint Monitor |
| :--- | :---: | :---: | :---: | :---: |
| **USB-C Wired** | `0x1D57` | `0x2112` | UsagePage `0x000B`, Usage `0x0000` | UsagePage `0x000A`, Usage `0x0000` |
| **Wireless 2.4GHz** | `0x1D57` | `0xFA60` | UsagePage `0x000B`, Usage `0x0000` | UsagePage `0x000A`, Usage `0x0000` |

### Ringkasan Paket Feature Report (`HidD_SetFeature`)
- `0x0C` (10 byte) — Handshake inisialisasi / commit profil.
- `0x04` (56 byte) — Stage DPI (tabel register PAW3311), Ripple Control, Angle Snapping, dan checksum 16-bit Big-Endian.
- `0x05` (15 byte) — Sleep Timer, Deep Sleep Timer, dan Key Debounce.
- `0x06` (9 byte) — Polling Rate (`125Hz=0x08`, `250Hz=0x04`, `500Hz=0x02`, `1000Hz=0x01` + komplemen bit).
- `0x08` (59 byte) — Mapping 18 slot tombol (3 byte per slot + checksum).
- `0x09` (3 × 64 byte) — Upload makro per slot tombol.
- **Input Monitor (`0x03 / 0xB3`)** — Laporan live dari mouse untuk persentase baterai, status *charging*, dan perpindahan level DPI fisik.

---

## Cara Build & Menjalankan

### Prasyarat
- **Windows 10 / 11 (x64)**
- **Rust Toolchain** (`rustc` & `cargo` dengan target `x86_64-pc-windows-msvc`)
- **Visual Studio Build Tools** (dengan Windows SDK untuk `rc.exe` guna menanamkan icon `.exe`)

### Build dari Source
```bash
git clone https://github.com/<username>/<repo>.git
cd atlas-control

# Jalankan unit test protokol HID
cargo test

# Build binary release teroptimasi (LTO + strip)
cargo build --release
```

Hasil binary akan tersedia di:
```text
target/release/PressPlayATLAS.exe
```

> **Catatan:** Tutup terlebih dahulu software bawaan `Mouse.exe` dari *system tray* Windows sebelum menjalankan `PressPlayATLAS.exe` agar tidak terjadi perebutan akses USB HID.

---

## Struktur Project

```text
atlas-control/
├── assets/
│   ├── Icon.ico            # Icon aplikasi Press Play ATLAS
│   └── app.rc              # Resource script icon Windows EXE
├── src/
│   ├── main.rs             # Entry point, window config & decoder ICO
│   ├── model.rs            # Struktur profil, DPI, tombol, makro & telemetry
│   ├── storage.rs          # Penyimpanan JSON (%APPDATA%\AtlasControl\config.json)
│   ├── hid/
│   │   ├── protocol.rs     # Builder paket HID, LUT PAW3311 & parser monitor + unit test
│   │   ├── win32.rs        # FFI native Windows HID (hid.dll, setupapi.dll, kernel32.dll)
│   │   └── manager.rs      # Background worker thread & auto-reconnect USB
│   └── ui/
│       ├── theme.rs        # Tema dark obsidian, Segoe UI / Cascadia Mono & custom widgets
│       ├── mouse_viz.rs    # Visualizer vektor mouse interaktif + live click/scroll feedback
│       ├── views.rs        # Halaman Tombol, Sensor & DPI, Makro (All Drag), Daya & Profil
│       └── app.rs          # Shell utama, header pill nav, toast & watermark
├── build.rs                # Otomatis embed Icon.ico ke binary .exe via rc.exe
└── Cargo.toml
```

---

## Penyimpanan Konfigurasi

Semua profil, makro, dan preferensi tema disimpan secara lokal dalam format JSON yang mudah dibaca di:
```text
%APPDATA%\AtlasControl\config.json
```

---

## Penafian (*Disclaimer*)

Project ini adalah aplikasi komunitas/independen hasil reverse-engineering protokol USB HID dan **tidak berafiliasi secara resmi dengan Press Play**. Gunakan dengan bijak dan tanggung jawab masing-masing.
