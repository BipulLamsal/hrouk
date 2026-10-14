async function load() {
  const res = await fetch("/users");
  const users = await res.json();
  document.getElementById("rows").innerHTML = users
    .map(
      (u) => `<tr>
        <td>${u.email}</td>
        <td><code>${u.endpoint_id.slice(0, 16)}…</code></td>
        <td>${u.display_name}</td>
      </tr>`
    )
    .join("");

  const sres = await fetch("/status");
  const rows = await sres.json();
  document.getElementById("status").innerHTML = rows
    .map((s) => {
      const state = s.last_event
        ? `${s.online ? "🟢 online" : "⚪ offline"} (${s.last_event})`
        : "never seen";
      const auth = s.authorized ? "authorized" : "unknown";
      return `<tr>
        <td>${s.email}</td>
        <td>${state} · ${auth}</td>
        <td>${s.at ?? "—"}</td>
      </tr>`;
    })
    .join("");
}

document.getElementById("f").onsubmit = async (e) => {
  e.preventDefault();
  const query = new URLSearchParams(new FormData(e.target)).toString();
  const res = await fetch("/register?" + query);
  const msg = document.getElementById("msg");
  if (res.ok) {
    msg.textContent = "registered ✓";
    msg.className = "";
    e.target.reset();
    load();
  } else {
    msg.textContent = await res.text();
    msg.className = "err";
  }
};

load();
