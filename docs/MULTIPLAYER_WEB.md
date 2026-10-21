# Multiplayer cho bản web — kế hoạch tích hợp

Tài liệu này ghi lại tình trạng đã khảo sát trong source (2026-10-21) và các
bước còn lại để bản web chơi online được, dùng server relay ở thư mục
`server/` (deploy Railway).

## Hiện trạng trong source gốc

- UI + logic **Online/LAN vẫn còn nguyên** trong game
  (`src/ui/network_game.rs`, màn hình tạo/vào phòng, danh sách người chơi).
  Trên web nó bị ẩn bởi 1 dòng `#[cfg(not(target_arch = "wasm32"))]` ở nút
  Online trong `src/ui/main_menu.rs`.
- Lý do ẩn: transport mặc định dùng **iroh (QUIC/UDP)** cho online và **mDNS**
  cho LAN — cả hai đều không tồn tại trong trình duyệt.
- Netcode: **GGRS** (`P2PSession`) chạy trên trait `NetworkSocket` của
  bones_framework; matchmaking theo giao thức của crate `bones_matchmaker`
  (xin ghép trận → được cấp `seed` + `player_idx` → các bên tự đồng bộ bằng
  deterministic simulation). Phần này **không phải viết lại**, chỉ cần thay
  transport bên dưới.

## Phần đã chuẩn bị

- `server/server.js`: relay + matchmaker WebSocket nói đúng luồng giao thức
  trên (accepted → success{seed, player_idx} → proxy 2 chiều →
  peer_disconnected), đã test tự động 2 client (`server/test-client.js`).
- Client web sẽ trỏ tới `wss://<domain-railway>/ws`.

## Các bước còn lại phía client (giai đoạn tiếp theo)

1. **WebSocket `NetworkSocket` cho wasm32** (trong jumpy hoặc bones_framework):
   - Kết nối `web_sys::WebSocket` tới relay; hiện thực các hàm của trait
     `NetworkSocket` (`send`, `recv`, `peer_addr`...) bằng cách dịch qua lại
     giữa packet của GGRS (bytes) và message `proxy`/`proxy_msg` (base64)
     của relay.
   - Hiện thực phía matchmaking: gửi `request_matchmaking`, chờ `success`
     lấy `seed`/`player_idx` — thay cho `OnlineMatchmaker` chạy QUIC ở
     `bones_framework::networking::online_matchmaking`.
2. **Mở lại mục Online cho wasm**: thay điều kiện `cfg` ở `main_menu.rs`
   bằng nhánh riêng cho wasm dùng transport WebSocket (giữ nguyên nhánh
   iroh cho bản desktop).
3. **Cho phép nhập địa chỉ server** trên UI (mặc định là domain Railway đã
   deploy), lưu vào localStorage phía vỏ HTML hoặc biến môi trường lúc build.
4. **Kiểm chứng**: 2 trình duyệt (hoặc điện thoại + máy tính) vào cùng 1 trận
   online, chơi hết 1 ván, kiểm tra đồng bộ (desync sẽ tự lộ vì GGRS checksum).

## Ghi chú kỹ thuật

- WebSocket là TCP: mọi gói đều "reliable + ordered" — GGRS vẫn chạy tốt
  (đổi lại độ trễ biến thiên mạnh hơn UDP; input delay mặc định của game sẽ
  gánh phần này ở giai đoạn prototype).
- Seed phải truyền dạng **chuỗi** vì là số u64 (đã xử lý ở server).
- Relay chỉ chuyển tiếp, không chứa logic game: chi phí vận hành rất thấp,
  1 instance Railway bản nhỏ đủ cho thử nghiệm; khi cần scale thì tách
  matchmaker và relay hoặc thêm phòng theo `match_data`.
