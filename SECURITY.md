# Security policy

## Reporting a vulnerability

Please **do not open a public issue** for security problems. Report them privately through
GitHub: **Security → Report a vulnerability** on
[realrevi/agentflow](https://github.com/realrevi/agentflow/security/advisories/new).
Include the AgentFlow version, your OS, what an attacker could do and how to reproduce it.

You can expect a first answer within a week. Fixes ship in a new release, and the advisory is
published once users have had a chance to update. Only the latest release is supported.

## How AgentFlow protects you

- **API keys** are stored only in your operating system's keychain (Windows Credential
  Manager, macOS Keychain, Secret Service on Linux). They are never written to settings,
  logs, error messages, or sent to the app's interface; the UI only sees whether a key is set
  and its last four characters. Requests do not follow redirects, and provider URLs must use
  HTTPS (plain HTTP only for your own machine).
- **API agents** work only inside the open workspace folder. Writing files and running
  commands ask for your permission by default; "allow for this conversation" is per tool, and
  **dangerous commands always ask**, whatever you allowed before. Only a real click or key
  press on the permission card can answer it; AgentFlow never answers one on its own. File
  changes are checkpointed so you can undo them.
- **Agent CLIs** (Claude Code, Codex, Gemini CLI) run in normal terminals with their own
  permission systems. Restored panels never start an agent on their own.
- **Community library** items are scanned for risky patterns and shown in full before
  installing; nothing from the library is ever run by AgentFlow.
- **Updates** are downloaded from GitHub Releases and installed only when you click
  "Install and restart", after their signature is checked against the public key built into
  the app.
