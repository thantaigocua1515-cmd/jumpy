# jumpy-relay — server multiplayer cho bản web

Server matchmaker + relay qua **WebSocket** cho bản web của game
[fishfolk/jumpy](https://github.com/fishfolk/jumpy) (fork: thantaigocua1515-cmd/jumpy).

Vì sao cần server này: netcode của game (GGRS) chạy trên giao thức matchmaker
của bones_framework, nhưng transport gốc dùng QUIC/UDP (iroh) + mDNS nên
**trình duyệt không dùng được**. Server này nói lại đúng luồng matchmaking đó
trên WebSocket:

```
Client → request_matchmaking { game_id, player_count, match_data }
Server → accepted                       (đã vào hàng chờ)
Server → success { seed, player_idx, player_count }   (đủ người, bắt đầu)
Client → proxy { to: idx|"all", reliable, data: "<base64>" }
Server → proxy_msg { from, reliable, data }           (chuyển cho đối thủ)
Server → peer_disconnected { player_idx }             (đối thủ thoát)
Client → stop_matchmaking / ping → pong
```

`seed` là số 64-bit dạng chuỗi thập phân (JSON number không chứa nổi u64).
Nhiệm vụ của server chỉ là ghép người + chuyển tiếp gói; logic game vẫn chạy
deterministic ở 2 đầu như thiết kế GGRS gốc.

## Chạy local

```bash
npm install
npm start          # lắng nghe ở PORT (mặc định 8931)
npm test           # test tự động 2 client: ghép trận + proxy 2 chiều
```

Kiểm tra sống: `GET /` trả JSON `{ service: "jumpy-relay", ok: true, ... }`.

## Deploy lên Railway

1. Vào [railway.app](https://railway.app) → **New Project** → **Deploy from
   GitHub repo** → chọn repo **thantaigocua1515-cmd/jumpy**.
2. Trong phần Settings của service vừa tạo:
   - **Root Directory:** `server`
   - (Railway tự nhận Node.js qua `package.json`, lệnh chạy là `npm start`.)
3. Railway tự cấp biến `PORT` — server đã đọc sẵn, không cần cấu hình thêm.
4. Vào **Settings → Networking → Generate Domain** để lấy địa chỉ public,
   dạng `https://jumpy-relay-production-xxxx.up.railway.app`.
   Địa chỉ WebSocket của client sẽ là `wss://jumpy-relay-production-xxxx.up.railway.app/ws`.
5. Mở thử `https://<domain>/` trên trình duyệt: thấy JSON `ok: true` là server sống.

Chi phí: server rất nhẹ (chỉ chuyển tiếp gói nhỏ), gói Hobby của Railway đủ
cho giai đoạn thử nghiệm 2 người chơi.

## Trạng thái tích hợp

- [x] Server relay/matchmaker (repo này, đã test local 2 client)
- [ ] Client web: viết `NetworkSocket` trên WebSocket cho target wasm32 và mở
      lại mục Online trên bản web — xem `docs/MULTIPLAYER_WEB.md` ở thư mục
      gốc của repo để biết kế hoạch chi tiết từng bước.
