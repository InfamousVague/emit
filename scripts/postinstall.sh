#!/bin/bash
# Post-install script — checks for optional system dependencies
# Runs automatically after `npm install`

set -e

BOLD="\033[1m"
DIM="\033[2m"
GREEN="\033[32m"
YELLOW="\033[33m"
CYAN="\033[36m"
RESET="\033[0m"

echo ""
echo -e "${BOLD}emit${RESET} — checking system dependencies..."
echo ""

# Track missing deps
missing=()

# ── terminal-notifier (clickable macOS notifications) ──────────────────────────
if command -v terminal-notifier &>/dev/null; then
  echo -e "  ${GREEN}✓${RESET} terminal-notifier"
else
  echo -e "  ${YELLOW}✗${RESET} terminal-notifier ${DIM}(clickable notifications)${RESET}"
  missing+=("terminal-notifier")
fi

# ── Rust toolchain ─────────────────────────────────────────────────────────────
if command -v cargo &>/dev/null; then
  echo -e "  ${GREEN}✓${RESET} cargo ($(cargo --version 2>/dev/null | cut -d' ' -f2))"
else
  echo -e "  ${YELLOW}✗${RESET} cargo ${DIM}(Rust toolchain)${RESET}"
  missing+=("rust")
fi

echo ""

# ── Offer to install missing deps ─────────────────────────────────────────────
if [ ${#missing[@]} -eq 0 ]; then
  echo -e "${GREEN}All system dependencies are installed.${RESET}"
  echo ""
  exit 0
fi

# Check if brew is available
if ! command -v brew &>/dev/null; then
  echo -e "${DIM}Install missing dependencies manually:${RESET}"
  for dep in "${missing[@]}"; do
    case "$dep" in
      terminal-notifier) echo -e "  brew install terminal-notifier" ;;
      rust) echo -e "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" ;;
    esac
  done
  echo ""
  exit 0
fi

# Interactive prompt — only if running in a TTY
if [ -t 0 ]; then
  echo -e "${CYAN}Install missing dependencies via Homebrew?${RESET}"
  echo -e "${DIM}This is optional — the app works without them but with reduced functionality.${RESET}"
  echo ""

  for dep in "${missing[@]}"; do
    case "$dep" in
      terminal-notifier)
        read -r -p "  Install terminal-notifier? (clickable notifications) [y/N] " answer
        if [[ "$answer" =~ ^[Yy]$ ]]; then
          brew install terminal-notifier
          echo -e "  ${GREEN}✓${RESET} terminal-notifier installed"
        else
          echo -e "  ${DIM}Skipped${RESET}"
        fi
        ;;
      rust)
        echo -e "  ${DIM}Rust must be installed manually:${RESET}"
        echo -e "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        ;;
    esac
  done
else
  # Non-interactive — just print instructions
  echo -e "${DIM}Optional dependencies (install for full functionality):${RESET}"
  for dep in "${missing[@]}"; do
    case "$dep" in
      terminal-notifier) echo -e "  brew install terminal-notifier  ${DIM}# clickable notifications${RESET}" ;;
      rust) echo -e "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  ${DIM}# Rust toolchain${RESET}" ;;
    esac
  done
fi

echo ""
