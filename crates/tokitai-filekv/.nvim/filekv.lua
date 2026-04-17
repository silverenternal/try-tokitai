-- tokitai-filekv project-specific Neovim configuration
-- Source this from your init.lua or add to your plugins/rust.lua
--
-- Usage: Add this to your plugins/rust.lua as a project override:
--   vim.lsp.config("rust_analyzer", { ... settings below ... })

-- Rust Analyzer project-specific settings for tokitai-filekv
-- These override your global rust_analyzer settings when editing this project

local rust_analyzer_settings = {
  ["rust-analyzer"] = {
    -- Enable all features for complete code analysis
    cargo = {
      features = "all",
      loadOutDirsFromCheck = true,
      buildScripts = {
        enable = true,
      },
    },
    -- Diagnostics
    diagnostics = {
      enable = true,
      disabled = {
        "inactive-code",  -- Too noisy with many #[cfg] features
      },
      warningsAsHint = {
        "missing-docs",
      },
    },
    -- Inlay hints (tailored for this codebase)
    inlayHints = {
      bindingModeHints = { enable = false },
      chainingHints = { enable = true },
      closingBraceHints = { enable = true, minLines = 15 },
      closureCaptures = { enable = true },
      closureReturnTypeHints = { enable = "with_block" },
      discriminantHints = { enable = "never" },
      lifetimeElisionHints = { enable = "never" },
      maxLength = 30,
      parameterHints = { enable = true },
      rangeExclusiveHints = { enable = true },
    },
    -- Proc macro
    procMacro = {
      enable = true,
      ignored = {
        ["thiserror-impl"] = { "thiserror_impl" },  -- Speed up analysis
      },
    },
    -- Import granularity
    imports = {
      granularity = {
        group = "crate",
      },
      prefix = "crate",
    },
    -- Assist settings
    assist = {
      importGranularity = "crate",
      importEnforceGroup = true,
      importPrefix = "crate",
    },
  },
}

-- Project-specific keymaps (add to your on_attach function)
local function filekv_on_attach(client, bufnr)
  local map = function(mode, lhs, rhs, opts)
    opts = opts or {}
    opts.buffer = bufnr
    vim.keymap.set(mode, lhs, rhs, opts)
  end

  -- Test runners
  map("n", "<leader>ta", "<cmd>!cargo nextest run --all-features<cr>", { desc = "Run all tests (nextest)" })
  map("n", "<leader>tf", "<cmd>!cargo nextest run --all-features --no-fail-fast<cr>", { desc = "Run all tests (no fail fast)" })
  map("n", "<leader>tl", "<cmd>!cargo test --all-features -- --list<cr>", { desc = "List all tests" })

  -- Clippy & format
  map("n", "<leader>lx", "<cmd>!cargo clippy --all-features --all-targets -- -D warnings<cr>", { desc = "Full clippy check" })

  -- Build shortcuts
  map("n", "<leader>ba", "<cmd>!cargo build --all-features<cr>", { desc = "Build all features" })
  map("n", "<leader>br", "<cmd>!cargo build --release --all-features<cr>", { desc = "Release build" })

  -- Benchmark
  map("n", "<leader>bb", "<cmd>!cargo bench --features benchmarks<cr>", { desc = "Run benchmarks" })

  -- Clean test data
  map("n", "<leader>cd", "<cmd>!rm -rf segments/* index/* wal/* checkpoints/*<cr>", { desc = "Clean test data" })
end

return {
  rust_analyzer_settings = rust_analyzer_settings,
  on_attach = filekv_on_attach,
}
