# agent-spec CLI Command Reference

## All Commands

```
agent-spec <COMMAND>

Commands:
  parse               Parse .spec/.spec.md files and show AST
  lint                Analyze spec quality (detect smells)
  verify              Verify code against specs
  init                Create a starter .spec.md file
  lifecycle           Run full lifecycle: lint -> verify -> report
  brief               Compatibility alias for the contract view
  contract            Render an explicit Task Contract for agent execution
  guard               Git guard: lint all specs + verify against change scope
  explain             Generate a human-readable contract review summary
  stamp               Preview git trailers for a verified contract
  checkpoint          Preview or create a VCS checkpoint
  plan                Generate structured plan context from spec + codebase scan
  resolve-ai          Merge external AI decisions into a verification report
  measure-determinism [Experimental] Measure contract verification determinism
  install-hooks       Install git hooks for automatic spec checking
```

## Core Flow

```bash
# 1. Read the contract
agent-spec contract specs/task.spec

# 2. Generate plan context for AI
agent-spec plan specs/task.spec --code . --format prompt

# 3. Implement code...

# 4. Verify
agent-spec lifecycle specs/task.spec --code . --format json

# 5. Repo-wide guard
agent-spec guard --spec-dir specs --code .
```

## plan

```bash
agent-spec plan <spec> [--code .] [--format text|json|prompt] [--depth shallow|full]
```

Generates structured plan context by combining three blocks:
- **Contract** — from `TaskContract::from_resolved()` (same as `contract` command)
- **Codebase Context** — scans Allowed Changes paths for file summaries, pub signatures, test functions
- **Task Sketch** — groups scenarios by dependency order (topological sort)

Options:
- `--format text` (default): human-readable structured summary
- `--format json`: machine-parseable with `contract`, `codebase_context`, `task_sketch` fields
- `--format prompt`: self-contained AI prompt (includes all inherited constraints)
- `--depth shallow` (default): file names + first-line summaries
- `--depth full`: includes `pub fn`/`pub struct`/`pub enum`/`pub trait` signatures

Respects `.gitignore`. Warns (does not error) on missing Allowed Changes paths.

## contract

```bash
agent-spec contract <spec> [--format text|json]
```

Renders the Task Contract with: Intent, Must/Must NOT, Decisions, Boundaries, Completion Criteria.

## lifecycle

```bash
agent-spec lifecycle <spec> --code <dir> \
  [--change <path>]... \
  [--change-scope none|staged|worktree|jj] \
  [--ai-mode off|stub] \
  [--min-score 0.6] \
  [--format text|json|md] \
  [--run-log-dir <dir>] \
  [--adversarial] \
  [--layers lint,boundary,test,ai]
```

Full pipeline: lint -> verify -> report. Default format is `json`.

Task specs can choose a runner in frontmatter:

```yaml
runner: cargo | maven | gradle | android | ios | node
runner_config: { scheme: "IosMini", destination: "platform=iOS Simulator,name=iPhone 16 Pro" }
```

Runner-specific behavior:

- `cargo`: detects `Cargo.toml`; runs `cargo test -q`.
- `maven`: detects `pom.xml`; prefers `./mvnw`; runs `test -Dtest=<filter>`.
- `gradle`: detects `build.gradle` or `build.gradle.kts`; prefers `./gradlew`; runs `test --tests <filter>`.
- `android`: detects Gradle plus `AndroidManifest.xml`; `Test.level: instrumented` uses connected-device preflight.
- `ios`: detects `Package.swift` or `*.xcodeproj`; macOS-only; uses `runner_config.scheme` and `runner_config.destination` for `xcodebuild test`.
- `node`: detects `package.json`; runs JavaScript/TypeScript package scripts with `npm`, `pnpm`, `yarn`, or `bun`. Use `runner: node` for TanStack Start, Vite, Vitest, Jest, Playwright, Bun, and other package-script based projects; there is no TanStack Start-specific runner.

Node runner v1 details:

- Package manager precedence: `runner_config.package_manager` > `package.json.packageManager` > a single lockfile marker > `npm`.
- Supported package managers: `npm`, `pnpm`, `yarn`, `bun`.
- Lockfiles: `pnpm-lock.yaml`, `bun.lock`, `bun.lockb`, `yarn.lock`, `package-lock.json`.
- Config keys: `package_manager`, `unit_script`, `typecheck_script`, `lint_script`, `build_script`, `e2e_script`, `unit_filter_style`, `workspace_filter`.
- Default scripts: `unit` -> `test`, `typecheck` -> `typecheck`, `lint` -> `lint`, `build` -> `build`, `e2e` -> `e2e`.
- `unit_filter_style` values: `vitest`, `jest`, `playwright`, `none`.
- Unit filters are regex-escaped before being passed to the package script.
- Non-unit levels require `Filter: -`.
- `Package` selectors and `runner_config.workspace_filter` fail in Node v1.
- Missing `package.json`, unreadable or invalid `package.json`, missing required scripts, invalid package-manager values, and ambiguous lockfiles fail verification.

When a required external tool or device is unavailable, runner preflight reports `MissingCapability`; `TestVerifier` converts that scenario to `skip`. For Node, this applies to the selected package manager executable and to opt-in `Level: e2e` browser capability.

## guard

```bash
agent-spec guard \
  [--spec-dir specs] \
  [--code .] \
  [--change <path>]... \
  [--change-scope staged|worktree] \
  [--min-score 0.6]
```

Scans all `*.spec` and `*.spec.md` files in `--spec-dir`, runs lint + verify on each. Default change scope is `staged`.

## verify

```bash
agent-spec verify <spec> --code <dir> \
  [--change <path>]... \
  [--change-scope none|staged|worktree] \
  [--ai-mode off|stub] \
  [--format text|json|md]
```

Raw verification without lint quality gate. Default change scope is `none`.

## explain

```bash
agent-spec explain <spec> \
  [--code .] \
  [--format text|markdown] \
  [--history]
```

Human-readable contract review summary. Use `--format markdown` for PR descriptions. Use `--history` to include run log history. In jj repos, `--history` also shows file-level diffs between adjacent runs via operation IDs.

## stamp

```bash
agent-spec stamp <spec> [--code .] [--dry-run]
```

Preview git trailers (`Spec-Name`, `Spec-Passing`, `Spec-Summary`). Currently only `--dry-run` is supported.

In jj repositories, also outputs `Spec-Change:` trailer with the current jj change ID.

## lint

```bash
agent-spec lint <files>... [--format text|json|md] [--min-score 0.0]
```

Built-in linters: VagueVerb, Unquantified, Testability, Coverage, Determinism, ImplicitDep, ExplicitTestBinding, Sycophancy.

## init

```bash
agent-spec init [--level org|project|task] [--name <name>] [--lang zh|en|both]
```

## Change Set Defaults

| Command | `--change-scope` default |
|---------|-------------------------|
| verify | `none` |
| lifecycle | `none` |
| guard | `staged` |

## resolve-ai

```bash
agent-spec resolve-ai <spec> \
  [--code .] \
  --decisions <decisions.json> \
  [--format text|json]
```

Merges external AI decisions into a verification report. Used as step 2 of the caller mode protocol:
1. `lifecycle --ai-mode caller` emits pending requests to `.agent-spec/pending-ai-requests.json`
2. Agent analyzes scenarios and writes `ScenarioAiDecision` JSON
3. `resolve-ai` merges decisions, replacing Skip verdicts with AI verdicts

The decisions file format:
```json
[
  {
    "scenario_name": "scenario name",
    "model": "claude-agent",
    "confidence": 0.92,
    "verdict": "pass",
    "reasoning": "All steps verified"
  }
]
```

Cleans up `pending-ai-requests.json` after successful merge.

## AI Mode

- `off` (default) - No AI verification layer
- `stub` - Returns `uncertain` for all scenarios (testing/scaffolding)
- `caller` - Agent-as-verifier: emits `AiRequest` JSON, resolved via `resolve-ai`
- `external` - Reserved for host-injected `AiBackend` trait implementations

## Verification Layers

Use `--layers` to select which verification layers to run:

```bash
# Only lint and boundary checking
agent-spec lifecycle specs/task.spec --code . --layers lint,boundary

# Skip lint, run structural + boundary + test
agent-spec lifecycle specs/task.spec --code . --layers boundary,test
```

Available layers: `lint`, `boundary`, `test`, `ai`
