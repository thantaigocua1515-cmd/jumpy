# Fish Folk: Jumpy — Bản Web Tiếng Việt, Chơi Online Trên Trình Duyệt

Đây là bản fork của **Fish Folk: Jumpy** (game bắn súng 2D chiến thuật của đội Fish Folk), được đưa lên trình duyệt để chơi ngay không cần cài đặt:

- **Chơi trực tuyến** qua server ghép trận riêng (WebSocket relay) — không còn mục chơi tại chỗ, vào game là tìm trận online.
- **Tự đấu với máy (bot)** nếu chờ quá 30 giây mà không có người chơi nào khác.
- **Giao diện tiếng Việt** toàn bộ (dịch từ bản tiếng Anh gốc).
- **Hỗ trợ điện thoại**: joystick vuốt để di chuyển, nút cảm ứng để bắn/nhảy/nhặt/trượt.

## Chơi ngay

Mở link: **https://thantaigocua1515-cmd.github.io/jumpy-web/**

Lần đầu tải khoảng 39MB nên hơi lâu; các lần sau sẽ nhanh hơn nhiều nhờ bộ nhớ đệm. Vào game → **Chơi Trực Tuyến** → **Tìm Trận**.

## Cách chơi

Hai đến bốn người chơi đấu trong một đấu trường nhỏ: di chuyển, nhảy và ngắm bắn chỉ theo hai hướng trái/phải — thắng thua nằm ở di chuyển khéo và chọn vị trí.

**Bàn phím (người chơi 1):**

| Phím | Hành động |
| --- | --- |
| W A S D | Di chuyển / ngắm |
| Space | Nhảy |
| C | Bắn |
| V | Nhặt / thả vũ khí |
| B | Trượt |
| Esc | Tạm dừng |
| Enter | Bắt đầu / xác nhận |

**Điện thoại (màn hình cảm ứng):**

- Vuốt ở nửa trái màn hình để di chuyển/ngắm; hất nhanh lên trên để nhảy.
- Các nút BẮN / NHẢY / NHẶT / TRƯỢT ở bên phải.
- Chạm nhẹ vẫn bấm được các nút trong menu như bình thường.
- Nút 🎮 ở góc phải trên để bật/tắt nút cảm ứng, nút ⛶ để toàn màn hình. Nên xoay ngang máy khi chơi.

**Luồng một trận đấu:** Tìm Trận → (ghép được người thật hoặc hết 30 giây thì đấu bot) → chọn cá và mũ → sẵn sàng → người tạo trận chọn map → đấu.

## Server online

- Server ghép trận đang chạy trên Railway: `https://jumpy-production.up.railway.app` (WebSocket: `wss://jumpy-production.up.railway.app/ws`).
- Mã nguồn server nằm trong thư mục [`server/`](./server) của repo này; xem thêm [`docs/MULTIPLAYER_WEB.md`](./docs/MULTIPLAYER_WEB.md) để biết giao thức và cách tích hợp.
- Muốn dùng server riêng: deploy thư mục `server/` lên bất kỳ dịch vụ chạy Node.js nào (Railway, Render, VPS...), rồi mở game kèm tham số `?relay=wss://địa-chỉ-server-của-bạn/ws`.

## Chạy từ mã nguồn

Yêu cầu: [Rust](https://rustup.rs/) (đúng phiên bản trong `rust-toolchain`).

- **Bản native (máy tính):** `cargo run`
- **Bản web:** cần thêm target `wasm32-unknown-unknown` và `wasm-opt` (binaryen) trong PATH, sau đó chạy `./scripts/build-web.sh release`. Kết quả nằm ở `web-target/wasm-release/` — chỉ là file tĩnh, host ở đâu cũng được (GitHub Pages, Vercel, Cloudflare Pages...).
- Repo này dùng một bản vá của thư viện [bones](https://github.com/fishfolk/bones) (fork riêng, nhánh `jumpy-web`) để thêm transport WebSocket cho bản web và ưu tiên locale tiếng Việt; `Cargo.toml` đã trỏ sẵn vào fork đó, không cần chỉnh tay.

## Bản quyền

'Fish Folk: Jumpy' source code is dual-licensed under either

* MIT License (licenses/LICENSE-MIT or http://opensource.org/licenses/MIT)
* Apache License, Version 2.0 (licenses/LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

Fish Folk media assets are Copyright (c) 2020-2024 The Fish Folk Game & Spicy Lobster Developers and licensed as [CC BY-NC](https://creativecommons.org/licenses/by-nc/4.0/).

Font VT323 dùng cho chữ tiếng Việt trong game được cấp phép theo SIL Open Font License 1.1 (xem `assets/ui/VT323-LICENSE.txt`).

## Credits

- [Fish Folk: Jumpy Credits](./CREDITS.md)
- Input Icons: [Kadith's Icons](https://kadith.itch.io/kadiths-free-icons) by Kadith

Bản fork này (bản web, chơi online qua WebSocket, vỏ cảm ứng cho điện thoại, bản dịch tiếng Việt) do [thantaigocua1515-cmd](https://github.com/thantaigocua1515-cmd) thực hiện trên nền game gốc của Fish Folk.
