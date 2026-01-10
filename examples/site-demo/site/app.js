async function fetchGen() {
  const res = await fetch("/app/site", { method: "GET", cache: "no-store" });
  const gen = res.headers.get("x-ron-reload-gen") || "?";
  document.getElementById("gen").textContent = gen;
  document.getElementById("ts").textContent = new Date().toLocaleTimeString();
}

async function doReload() {
  await fetch("/app/site/reload", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: "{}",
  });
  await fetchGen();
}

document.getElementById("reload").addEventListener("click", doReload);
document.getElementById("refresh").addEventListener("click", fetchGen);

fetchGen().catch(() => {});
