# CI/CD

towl uses GitHub Actions for continuous integration and documentation deployment.

## Documentation Deployment

The `docs.yml` workflow builds and deploys the mdBook documentation to GitHub Pages.

**Trigger:** Pushes to `main` that modify files in `docs/` or the workflow file itself. Can also be triggered manually via `workflow_dispatch`.

**Pipeline:**

1. **Build** -- Installs mdBook, runs `mdbook build docs`, uploads the `docs/book/` directory as a Pages artifact
2. **Deploy** -- Deploys the artifact to GitHub Pages

**Permissions required:**

- `contents: read` -- Read repository files
- `pages: write` -- Deploy to GitHub Pages
- `id-token: write` -- Authenticate with Pages

The workflow uses concurrency control (`group: pages`, `cancel-in-progress: true`) to prevent overlapping deployments.

## Running Locally

Build and preview the documentation locally:

```bash
# Install mdBook
cargo install mdbook

# Build docs
mdbook build docs

# Serve with live reload
mdbook serve docs
```

The built documentation is output to `docs/book/`.
