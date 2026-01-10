# Contributing to runa

Thank you for your interest in contributing to **runa**! Contributions of all forms are welcome, including bug reports, feature requests, documentation improvements, and pull requests.

---

## Reporting Issues

If you find a bug, please open an issue and include:

- A clear title and description
- Steps to reproduce (if applicable)
- Expected vs. actual behavior
- Environment details (OS, Terminal, etc.)

---

## Feature Requests

Feature ideas are welcomed.  

Please explain:
- The problem your feature solves
- Why it benefits users
- Any prior context or related proposals

---

## Pull Requests

Before opening a pull request:

- Ensure your branch is up to date with `main`
- Provide a clear summary of the change
- Link related issues when applicable
- Keep PRs focused (small and scoped is ideal)

PR checklist:

- [ ] Code follows existing conventions
- [ ] Documentation updated if behavior changed
- [ ] No unrelated changes mixed in

Since the project is currently developed by a single developer, review turnaround times may vary. Feedback will aim to be constructive and collaborative.

---

## Fork the Repository

1. Fork the repository on GitHub:

2. Clone your fork locally:
    ```sh
    git clone https://github.com/<your-username>/runa.git
    cd runa
    ```

3. Add the upstream remote:
    ```sh
    git remote add upstream https://github.com/alexm-dev/runa.git
    ```

## Development Setup

1. Make sure you have the lastest stable Rust toolchain installed
    ```sh
    rustc --version
    cargo --version
    ```

2. Build runa and run the tests afterwards
    ```sh
    cargo build
    cargo test
    ```

## Submit your branch

1. Create a new branch for your changes
    ```sh
    git checkout -b your-branch-name
    ```

2. Make your changes and ensure that it alligns with the overall coding style of runa.
3. Commit your changes with a description
    ```sh
    git commit -m "feat: new feature"
    ```

4. Push your changes to your fork
    ```sh
    git push origin your-branch-name
    ```


## Keep your Fork in sync.

Before Submittting your Pull Request, ensure your branch is up-to-date with the latest changes in runa.

1. Fetch latest changes
    ```sh
    git fetch upstream
    ```

2. Update your local main branch
    ```sh
    git checkout main
    git merge upstream/main
    ```

3. Rebase your feature branch
    ```sh
    git checkout your-branch-name
    git rebase main
    ```

---

## Communication

Design clarifications, ideas, and general discussion can happen via GitHub issues. Please be respectful and considerate toward other contributors.

---

## License

By contributing, you agree that your contributions will be licensed under the project's open-source license.

---

Thank you for helping improve **runa**!
