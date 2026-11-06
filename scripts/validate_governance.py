"""Dependency-free governance checks; no writes, network or application execution."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
ERRORS = []


def require(condition, message):
    if not condition:
        ERRORS.append(message)


def read_json(path):
    try:
        return json.loads((ROOT / path).read_text())
    except (OSError, ValueError) as exc:
        ERRORS.append(f"Cannot parse {path}: {type(exc).__name__}")
        return {}


def main():
    required = [
        'README.md', 'CONTRIBUTING.md', 'SECURITY.md', 'CODE_OF_CONDUCT.md',
        'AGENTS.md', '.editorconfig', '.gitignore',
        'docs/architecture/README.md', 'docs/decisions/README.md',
        *[f'docs/engineering/{name}.md' for name in
          ('git-workflow', 'coding-standards', 'testing-policy', 'release-policy')],
        '.github/PULL_REQUEST_TEMPLATE.md', '.github/workflows/governance.yml',
        *[f'.github/ISSUE_TEMPLATE/{name}.yml' for name in
          ('config', 'feature', 'bug', 'research', 'security')],
        'scripts/validate_governance.py',
    ]
    for name in required:
        path = ROOT / name
        require(path.is_file() and not path.is_symlink(), f'Missing/unsafe artifact: {name}')
        if not path.is_file() or path.is_symlink():
            continue
        with path.open(encoding='utf-8', newline='') as stream:
            text = stream.read()
        require(bool(text.strip()) and text.endswith('\n'), f'Empty/missing final newline: {name}')
        require('\r' not in text, f'Non-LF text: {name}')
        require(all(line == line.rstrip() for line in text.splitlines()), f'Trailing whitespace: {name}')
        if path.suffix == '.md':
            for target in re.findall(r'\]\(([^)]+)\)', text):
                if re.match(r'https?://|mailto:|#', target):
                    continue
                destination = (path.parent / target.split('#')[0]).resolve()
                require(destination.is_relative_to(ROOT) and destination.is_file(),
                        f'Broken/escaping local link in {name}')

    field_counts = {'feature': 8, 'bug': 7, 'research': 7}
    for name, count in field_counts.items():
        form = read_json(f'.github/ISSUE_TEMPLATE/{name}.yml')
        fields = [item for item in form.get('body', []) if item.get('type') == 'textarea']
        require(bool(form.get('name')) and bool(form.get('description')), f'Form metadata: {name}')
        require(len(fields) == count, f'Required field count: {name}')
        require(len({item.get('id') for item in fields}) == count, f'Unique field IDs: {name}')
        require(all(item.get('validations', {}).get('required') is True for item in fields),
                f'Required validations: {name}')
        require('labels' not in form, f'Unverified automatic labels: {name}')
    security = read_json('.github/ISSUE_TEMPLATE/security.yml')
    require(all(item.get('type') in ('markdown', 'checkboxes') for item in security.get('body', [])),
            'Security form must not solicit public exploit details')
    require(any(item.get('type') == 'checkboxes' for item in security.get('body', [])),
            'Security form needs a non-sensitive acknowledgement')
    config = read_json('.github/ISSUE_TEMPLATE/config.yml')
    require(config.get('blank_issues_enabled') is False, 'Blank Issues must be disabled')

    workflow = read_json('.github/workflows/governance.yml')
    require(workflow.get('permissions') == {'contents': 'read'}, 'Workflow token must be read-only')
    require(set(workflow.get('on', {})) == {'push', 'pull_request'}, 'Unsafe/unexpected workflow trigger')
    for trigger in workflow.get('on', {}).values():
        require(trigger.get('branches') == ['main'], 'Workflow target must be main')
    require(set(workflow.get('jobs', {})) == {'governance'}, 'Unexpected CI jobs')
    job = workflow.get('jobs', {}).get('governance', {})
    require(job.get('runs-on') == 'ubuntu-latest', 'Use ephemeral Linux runner')
    require(job.get('timeout-minutes') == 5, 'Missing bounded timeout')
    require('permissions' not in job and 'environment' not in job, 'Unexpected job privileges')
    steps = job.get('steps', [])
    require(len(steps) == 3, 'Unexpected workflow steps; review validator when changing CI')
    for step in steps:
        if 'uses' in step:
            require(bool(re.fullmatch(r'actions/checkout@[0-9a-f]{40}', step['uses'])), 'Unpinned Action')
            require(step.get('with', {}).get('persist-credentials') is False, 'Persisted credentials')
        if 'run' in step:
            require(step['run'] in ('python3 scripts/validate_governance.py', 'git show --format= --check HEAD'),
                    'Unexpected execution in governance workflow')
    require('secrets.' not in json.dumps(workflow), 'Secrets must not be referenced')

    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=ROOT).decode().split('\0')
    sensitive_directory = re.compile(r'(^|/)(secrets|credentials)/')
    sensitive = re.compile(r'(^|/)\.env($|\.)|\.(p12|pfx|p8|key|mobileprovision|provisionprofile)$')
    for name in filter(None, tracked):
        if sensitive_directory.search(name):
            require(False, 'Potential sensitive tracked filename (redacted)')
            continue
        if re.fullmatch(r'\.env(?:\..*)?\.example', Path(name).name):
            continue
        require(not sensitive.search(name), 'Potential sensitive tracked filename (redacted)')

    if os.environ.get('GITHUB_EVENT_NAME') == 'pull_request':
        event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
        pr = event['pull_request']
        types = r'feat|fix|docs|test|refactor|perf|build|ci|chore|security'
        require(bool(re.fullmatch(rf'({types})\([a-z0-9-]+\): \S[^\n]*', pr['title'])), 'PR title convention')
        branch = pr['head']['ref']
        match = re.fullmatch(r'(feat|fix|spike|chore|docs|test|security)/([1-9][0-9]*)-[a-z0-9]+(?:-[a-z0-9]+)*', branch)
        release = re.fullmatch(r'release/v?[0-9]+\.[0-9]+\.[0-9]+(?:-[a-z0-9.-]+)?', branch)
        require(bool(match or release), 'Branch convention')
        linked = re.findall(r'(?im)^\s*(?:Refs|Closes) #([1-9][0-9]*)\b', pr.get('body') or '')
        require(bool(linked), 'PR must link an Issue using Refs/Closes')
        if match:
            require(match.group(2) in linked, 'Branch Issue must match a PR Issue link')
    if ERRORS:
        print('\n'.join('FAIL: ' + error for error in ERRORS))
        return 1
    print(f'PASS: {len(required)} governance artifacts; links, forms, CI constraints and tracked filename checks')
    return 0


if __name__ == '__main__':
    sys.exit(main())
