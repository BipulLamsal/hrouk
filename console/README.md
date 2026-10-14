# console

Watches the LAN, checks agents against MySQL, and serves the HR web page.

Run:

    docker compose up -d
    cargo run -p console

Open http://127.0.0.1:8080 to enroll users and see who is online. Presence is in presence_log. A logout is written after the stale timeout or mDNS expiry.
