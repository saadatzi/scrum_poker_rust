// Determine the correct protocol: wss for HTTPS, ws for local HTTP
const protocol = window.location.protocol === "https:" ? "wss" : "ws";
const roomId = window.location.pathname.split("/").filter(Boolean)[1];
const ws = new WebSocket(
  `${protocol}://${window.location.host}/ws/${encodeURIComponent(roomId)}`,
);

let currentUserName = localStorage.getItem("poker_username") || "";

ws.onopen = () => {
  if (currentUserName) {
    document.getElementById("username").value = currentUserName;
    join();
  }
};

ws.onerror = (error) => {
  console.error("WebSocket Error:", error);
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  updateUI(data);
};

let notifications = [];
function showNotification(message) {
  const container = document.getElementById("notification");

  // Create message element
  const msgEl = document.createElement("div");
  msgEl.className = "notification-message";
  msgEl.innerText = message;

  // Add to internal list and DOM
  notifications.push(msgEl);
  container.appendChild(msgEl);

  // Keep only latest 3
  if (notifications.length > 3) {
    const old = notifications.shift();
    if (old && old.parentNode) {
      old.remove();
    }
  }

  // Set timeout to remove after 5 seconds
  setTimeout(() => {
    msgEl.style.opacity = "0";
    setTimeout(() => {
      if (msgEl.parentNode) {
        msgEl.remove();
        notifications = notifications.filter((n) => n !== msgEl);
      }
    }, 500);
  }, 5000);
}

function updateUI(data) {
  if (data.notification) {
    showNotification(data.notification);
  }

  const revealBtn = document.getElementById("reveal-btn");
  if (revealBtn) revealBtn.innerText = data.revealed ? "Hide" : "Show";

  const list = document.getElementById("user-list");
  list.innerHTML = `
    <table id="user-table">
        <thead>
            <tr>
                <th>User</th>
                <th>Vote</th>
            </tr>
        </thead>
        <tbody id="user-table-body"></tbody>
    </table>
  `;

  const tbody = document.getElementById("user-table-body");

  // copy users so we can sort without mutating original
  let users = Array.isArray(data.users) ? data.users.slice() : [];

  // When votes are revealed, sort descending by numeric vote value.
  // Non-numeric votes (e.g. "?" or null) are placed at the end.
  if (data.revealed) {
    users.sort((a, b) => {
      const toNum = (v) => {
        if (v == null) return NaN;
        const n = Number(String(v).trim());
        return Number.isFinite(n) ? n : NaN;
      };
      const an = toNum(a.vote);
      const bn = toNum(b.vote);
      const aIsNaN = Number.isNaN(an);
      const bIsNaN = Number.isNaN(bn);
      if (aIsNaN && bIsNaN) return 0;
      if (aIsNaN) return 1; // push non-numeric to the end
      if (bIsNaN) return -1;
      return bn - an; // descending
    });
  }

  users.forEach((user) => {
    const tr = document.createElement("tr");
    const isMe = user.name === currentUserName;
    tr.innerHTML = `
        <td class="user-name-cell ${isMe ? "is-me" : ""}">
            ${user.name}
            ${isMe ? '<span class="edit-icon" onclick="editName()">✏️</span>' : ""}
        </td>
        <td class="user-vote">${
          data.revealed ? user.vote || "-" : user.vote ? DUCK_SVG : "-"
        }</td>
    `;
    tbody.appendChild(tr);
  });
}

function toggleReveal() {
  ws.send(JSON.stringify({ type: "reveal" }));
}

function clearVotes() {
  ws.send(JSON.stringify({ type: "clear" }));
}

function editName() {
  document.getElementById("poker-area").classList.add("hidden");
  document.getElementById("login-form-wrapper").classList.remove("hidden");
  document.getElementById("username").focus();
}

// Removed the card generation from the global scope.
// It will be triggered after a successful join.

function join() {
  const name = document.getElementById("username").value;
  if (!name) return;

  currentUserName = name;
  localStorage.setItem("poker_username", name);

  ws.send(JSON.stringify({ type: "join", name }));

  // Show Welcome Message
  const welcome = document.getElementById("welcome-msg");
  welcome.innerText = `Welcome, ${name}!`;
  welcome.classList.remove("hidden");
  setTimeout(() => {
    welcome.style.opacity = "0";
    setTimeout(() => welcome.classList.add("hidden"), 1000);
  }, 5000);

  document.getElementById("login-form-wrapper").classList.add("hidden");
  document.getElementById("poker-area").classList.remove("hidden");

  generateCards();
}

function generateCards() {
  const cardsDiv = document.getElementById("cards");
  if (cardsDiv.innerHTML !== "") return; // Already generated

  const VOTE_VALUES = [
    "0",
    "1",
    "2",
    "3",
    "5",
    "8",
    "13",
    "21",
    "34",
    "55",
    "89",
    "?",
  ];
  VOTE_VALUES.forEach((val) => {
    const btn = document.createElement("button");
    btn.className = "card";
    btn.innerText = val;
    btn.onclick = () => ws.send(JSON.stringify({ type: "vote", value: val }));
    cardsDiv.appendChild(btn);
  });
}

const DUCK_SVG = `<svg viewBox="0 0 120 120" width="36" height="36" xmlns="http://www.w3.org/2000/svg">
  <ellipse cx="60" cy="100" rx="45" ry="8" fill="#d4e6f1" opacity="0.8"/>
  <ellipse cx="60" cy="100" rx="35" ry="5" fill="#a9cce3" opacity="0.6"/>
  <path d="M25,75 C25,100 85,100 95,75 C100,62 90,55 80,55 C70,55 60,60 50,60 C35,60 25,62 25,75 Z" fill="#f1c40f" />
  <path d="M25,75 C20,70 15,62 20,58 C25,54 32,60 30,70" fill="#f39c12" />
  <path d="M45,70 C45,60 70,60 75,70 C75,78 55,85 45,70 Z" fill="#f39c12" />
  <path d="M50,72 C50,65 68,65 72,72 C72,78 58,82 50,72 Z" fill="#e67e22" opacity="0.5" />
  <path d="M72,58 C72,58 75,45 80,45 C85,45 88,58 88,58 Z" fill="#f1c40f" />
  <circle cx="82" cy="40" r="18" fill="#f1c40f" />
  <circle cx="75" cy="46" r="4" fill="#e74c3c" opacity="0.4" />
  <circle cx="88" cy="36" r="3.5" fill="#2c3e50" />
  <circle cx="89.5" cy="34.5" r="1.2" fill="#ffffff" />
  <path d="M96,36 C105,34 112,39 110,43 C105,48 95,44 96,36 Z" fill="#e67e22" />
  <path d="M97,40 C101,40 106,41 108,43 C101,45 97,42 97,40 Z" fill="#d35400" opacity="0.6" />
</svg>`;
