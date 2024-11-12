# dmypyls

`dmypyls` is a language server for the mypy type checker's `dmypy` daemon. `dmypyls` manages the
lifecycle of the `dmypy` daemon and provides a language server interface to it.

I should mention that there is plenty of prior art in this space. See
[sileht/dmypy-ls](https://github.com/sileht/dmypy-ls) for one example. Also, the mypy folks seem to
want to convert `dmypy` to a language server. See [this issue](https://github.com/python/mypy/issues/10463).

## Installing dmypyls

Assuming you have a recent version of the rust toolchain installed, you should be able to `cargo
install dmypyls`. For now, you'll need to ensure that `mypy` is in your path while running `dmypyls`.

## Neovim Config

```lua
vim.api.nvim_create_autocmd({ "BufRead" }, {
  pattern = { "*.py" },
  group = vim.api.nvim_create_augroup("dmypyls-bufread", { clear = true }),
  callback = function(_)
    if vim.fn.executable("dmypyls") ~= 0 then
      -- We found an executable for dmypyls.
      vim.lsp.set_log_level(vim.log.levels.INFO)
      vim.lsp.start({
        name = "dmypyls",
        cmd = { "dmypyls", vim.api.nvim_buf_get_name(0) },
        root_dir = vim.fs.root(0, {
          ".git",
          "pyproject.toml",
          "setup.py",
          "mypy.ini"
        })
      }, { bufnr = 0, reuse_client = function(_, _) return false end })
    end
  end
})
```

### Strict Mypy config

In order for mypy usage to be helpful, I recommend ramping up the strictness of its checks. Here is
an example `mypy.ini` with relatively strict configuration.

```ini
[mypy]
strict_optional = True
show_error_codes = True
pretty = True
warn_redundant_casts = True
warn_unused_ignores = True
ignore_missing_imports = False
check_untyped_defs = True
disallow_untyped_defs = True
disallow_any_unimported = True
no_implicit_optional = True
warn_return_any = True
```
