# AI development skills

The [UI skill](../.agents/skills/rs-lvgl-ui/SKILL.md) and
[native integration skill](../.agents/skills/rs-lvgl-native/SKILL.md) are maintained
with the Rust API. Each folder is self-contained: its references resolve inside
the copied skill, while source navigation starts from the target application's
actual dependency. The skills do not assume their installed location is the
library checkout.

## Repository use

Codex discovers `.agents/skills` from the working directory up to the repository
root. Invoke `$rs-lvgl-ui` for screens and `$rs-lvgl-native` for raw integration,
or allow ordinary skill discovery. `agents/openai.yaml` supplies display metadata;
implicit invocation remains enabled.

Example requests:

```text
Use $rs-lvgl-ui to add a settings screen with a keyed list, a confirmation
modal and explicit keyboard navigation. Preserve the current touch behavior.

Use $rs-lvgl-native to move this application's LVGL configuration into its
own header and compile the generated font C file with matching raw bindings.
```

## Install into another workspace

From this checkout, install both folders into the user's skill directory:

```sh
python3 scripts/install-skills.py --dest ~/.agents/skills
```

For a project-specific installation, choose that project's `.agents/skills`:

```sh
python3 scripts/install-skills.py --dest ../application/.agents/skills --skill rs-lvgl-ui
```

The installer checks all requested destinations before copying and refuses existing
folders. Updating an existing installation requires an explicit review/replacement;
it never silently overwrites local changes. Older environments configured around
`$CODEX_HOME/skills` can pass that directory explicitly. No global installation is
performed by building or testing this repository.

Once these folders exist on a remote ref, Codex's `$skill-installer` can install a
skill from that repository and path, such as
`umbrella22/rs_lvgl` + `.agents/skills/rs-lvgl-ui`. Select a ref containing
the matching API. Local uncommitted files are available only through local copying.

## Versioning and distribution

The UI skill uses ordinary function constructors and `View<M>`. Install the skill
from the ref matching the application's dependency so examples, features and
resource-lifetime guidance stay aligned with the code.

Co-location makes API changes, examples and skill guidance reviewable together.
A separate repository is useful when skills need their own release cadence,
compatibility matrix or maintainers, or cover several unrelated LVGL wrappers.
Installation alone does not require that split. For bundled distribution with
multiple skills or connectors, official OpenAI guidance recommends plugins; plugin
packaging and publication are separate deliverables.

[Official skill documentation](https://developers.openai.com/codex/skills) defines
repository discovery, user locations and installation. The skills here use the
standard `SKILL.md` format and include no tool-server dependency.

## Maintenance checks

Validate `SKILL.md` frontmatter with the available skill-creator validator. Copy
both folders into a temporary directory, validate their relative references, run
the included [counter](../.agents/skills/rs-lvgl-ui/references/counter.rs) against
the actual reduced UI dependencies, and verify a repeated install refuses to
replace the first copy. These checks establish package and API usability;
independent agent behavioral evaluation is a separate check.
