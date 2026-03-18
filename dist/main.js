const { invoke } = window.__TAURI__.core;

const input = document.getElementById("search-input");
const resultsEl = document.getElementById("results");
let selectedIndex = 0;
let currentResults = [];

async function doSearch(query) {
  currentResults = await invoke("search", { query });
  selectedIndex = 0;
  render();
}

function render() {
  if (currentResults.length === 0) {
    resultsEl.innerHTML = '<div class="empty-state">No results</div>';
    return;
  }

  resultsEl.innerHTML = currentResults
    .map(
      (cmd, i) => `
    <div class="result-item ${i === selectedIndex ? "selected" : ""}" data-id="${cmd.id}" data-index="${i}">
      <div class="result-icon">${cmd.name[0]}</div>
      <div class="result-info">
        <div class="result-name">${cmd.name}</div>
        <div class="result-desc">${cmd.description}</div>
      </div>
      <div class="result-category">${cmd.category}</div>
    </div>
  `
    )
    .join("");
}

input.addEventListener("input", (e) => doSearch(e.target.value));

document.addEventListener("keydown", async (e) => {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    selectedIndex = Math.min(selectedIndex + 1, currentResults.length - 1);
    render();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex = Math.max(selectedIndex - 1, 0);
    render();
  } else if (e.key === "Enter" && currentResults[selectedIndex]) {
    await invoke("execute_command", { id: currentResults[selectedIndex].id });
    await invoke("hide_window");
  } else if (e.key === "Escape") {
    await invoke("hide_window");
  }
});

resultsEl.addEventListener("click", async (e) => {
  const item = e.target.closest(".result-item");
  if (item) {
    await invoke("execute_command", { id: item.dataset.id });
    await invoke("hide_window");
  }
});

// Load all commands on start
doSearch("");
