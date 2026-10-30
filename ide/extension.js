const vscode = require('vscode');
const { existsSync, writeFileSync, unlinkSync, statSync } = require('fs');
const { dirname, join, delimiter, relative } = require('path');
const { homedir, tmpdir } = require('os');
const { execFile } = require('child_process');

function findProjectDir(filePath) {
    if (!filePath) {
        if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
            return vscode.workspace.workspaceFolders[0].uri.fsPath;
        }
        return process.cwd();
    }
    let curr = dirname(filePath);
    for (let i = 0; i < 15; i++) {
        if (existsSync(join(curr, 'flame.toml'))) {
            return curr;
        }
        const parent = dirname(curr);
        if (parent === curr) break;
        curr = parent;
    }
    if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
        return vscode.workspace.workspaceFolders[0].uri.fsPath;
    }
    return dirname(filePath);
}

function findWorkspaceRoot(documentPath) {
    if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
        return vscode.workspace.workspaceFolders[0].uri.fsPath;
    }
    return dirname(documentPath);
}

function findCompilerBinary(startPath) {
    const existing = [];
    const addIfExist = (cand) => { if (cand && existsSync(cand)) existing.push(cand); };

    // 1. Check user configuration
    try {
        const configPath = vscode.workspace.getConfiguration('flame').get('compilerPath');
        if (configPath && typeof configPath === 'string' && configPath.trim() !== '') {
            if (existsSync(configPath)) return configPath;
        }
    } catch (e) { }

    // 2. Check workspace directories and parents
    let curr = startPath;
    for (let i = 0; i < 8; i++) {
        if (curr) {
            const candidates = [
                join(curr, 'target', 'release', 'flamelang.exe'),
                join(curr, 'target', 'debug', 'flamelang.exe'),
                join(curr, 'target', 'release', 'flamelang'),
                join(curr, 'target', 'debug', 'flamelang'),
            ];
            for (const cand of candidates) addIfExist(cand);

            const parent = dirname(curr);
            if (parent === curr) break;
            curr = parent;
        } else {
            break;
        }
    }

    // 3. Check sibling flame repository / workspace directories
    if (startPath) {
        const parentDir = dirname(startPath);
        const siblingCandidates = [
            join(parentDir, 'flame', 'target', 'release', 'flamelang.exe'),
            join(parentDir, 'flame', 'target', 'debug', 'flamelang.exe'),
            join(parentDir, 'flame', 'target', 'release', 'flamelang'),
            join(parentDir, 'flame', 'target', 'debug', 'flamelang'),
        ];
        for (const cand of siblingCandidates) addIfExist(cand);
    }

    // 4. Check Cargo global binary directory
    const userHome = homedir();
    const cargoCandidates = [
        join(userHome, '.cargo', 'bin', 'fmp.exe'),
        join(userHome, '.cargo', 'bin', 'fmp'),
    ];
    for (const cand of cargoCandidates) addIfExist(cand);

    // 5. Check NPM global directories (flame installed via npm)
    if (process.platform === 'win32') {
        const appData = process.env.APPDATA || '';
        const localAppData = process.env.LOCALAPPDATA || '';
        const npmCandidates = [
            join(appData, 'npm', 'fmp.cmd'),
            join(appData, 'npm', 'fmp.exe'),
            join(appData, 'npm', 'fmp'),
            join(localAppData, 'npm', 'fmp.cmd'),
            join(localAppData, 'npm', 'fmp.exe'),
            join(localAppData, 'npm', 'fmp'),
        ];
        for (const cand of npmCandidates) addIfExist(cand);
    } else {
        const unixNpmCandidates = [
            join(userHome, '.npm-global', 'bin', 'fmp'),
            join(userHome, '.npm-global', 'bin', 'fmp'),
            '/usr/local/bin/fmp',
            '/usr/local/bin/fmp',
            '/usr/bin/fmp',
            '/usr/bin/fmp',
        ];
        for (const cand of unixNpmCandidates) addIfExist(cand);
    }

    // 6. Check system PATH entries
    if (process.env.PATH) {
        const pathDirs = process.env.PATH.split(delimiter);
        const cliNames = process.platform === 'win32'
            ? ['fmp.cmd', 'fmp.exe', 'fmp.bat', 'fmp', 'fmp.cmd', 'fmp.exe', 'fmp.bat', 'fmp']
            : ['fmp', 'fmp'];

        for (const pDir of pathDirs) {
            for (const name of cliNames) {
                addIfExist(join(pDir, name));
            }
        }
    }

    // Return the newest binary by mtime timestamp so recent compiler rebuilds take precedence over older binaries
    if (existing.length > 0) {
        let newest = existing[0];
        let newestTime = 0;
        for (const cand of existing) {
            try {
                const mtime = statSync(cand).mtimeMs;
                if (mtime > newestTime) {
                    newestTime = mtime;
                    newest = cand;
                }
            } catch (e) { }
        }
        return newest;
    }

    // Default fallback to 'flame' CLI (supports npm / cargo / system PATH via shell invocation)
    return 'flame';
}

function execCompilerJson(args, cwd, input) {
    return new Promise((resolve) => {
        const compiler = findCompilerBinary(cwd);
        if (!compiler) {
            resolve(null);
            return;
        }

        let options = { cwd, maxBuffer: 10 * 1024 * 1024 };
        if (process.platform === 'win32' || !compiler.includes('/') && !compiler.includes('\\')) {
            options.shell = true;
        }

        const child = execFile(compiler, args, options, (error, stdout) => {
            if (error && !stdout) {
                resolve(null);
                return;
            }

            try {
                resolve(JSON.parse(stdout));
            } catch (e) {
                resolve(null);
            }
        });

        if (input !== undefined) {
            child.stdin.write(input);
            child.stdin.end();
        }
    });
}

async function runCheck(document, position) {
    if (!document || document.uri.fsPath.endsWith('.fmi') || document.uri.fsPath.endsWith('.tmp.fm')) return null;
    const workspaceRoot = findWorkspaceRoot(document.uri.fsPath);

    const args = ['check', document.uri.fsPath, '--json', '--stdin'];
    if (position) {
        args.push('--line', String(position.line + 1), '--col', String(position.character + 1));
    }

    const result = await execCompilerJson(args, workspaceRoot, document.getText());
    return result;
}

function removeTableBorders(markdown) {
    if (!markdown || typeof markdown !== 'string' || !markdown.includes('|')) return markdown;

    const lines = markdown.split('\n');
    const result = [];
    let inTable = false;

    for (let i = 0; i < lines.length; i++) {
        const trimmed = lines[i].trim();
        if (trimmed.startsWith('|') && trimmed.endsWith('|')) {
            const cells = trimmed
                .split('|')
                .map(c => c.trim())
                .filter((_, idx, arr) => idx > 0 && idx < arr.length - 1);

            // Separator row (| --- | --- |)
            if (cells.every(c => /^:?-+:?$/.test(c))) {
                inTable = true;
                continue;
            }

            // Header row
            if (!inTable) {
                inTable = true;
                continue;
            }

            // Data rows: format as clean borderless bullet items
            if (cells.length >= 3) {
                const col1 = cells[0].replace(/^`|`$/g, '');
                const col2 = cells[1].replace(/^`|`$/g, '');
                const col3 = cells.slice(2).join(' — ');
                result.push(`- \`${col1}: ${col2}\` — ${col3}`);
            } else if (cells.length === 2) {
                const col1 = cells[0].replace(/^`|`$/g, '');
                const col2 = cells[1];
                result.push(`- \`${col1}\`: ${col2}`);
            } else if (cells.length === 1) {
                result.push(`- \`${cells[0]}\``);
            }
        } else {
            inTable = false;
            result.push(lines[i]);
        }
    }

    return result.join('\n');
}

function toCompletionItem(entry, hasTrailingParen = false) {
    let kind = vscode.CompletionItemKind.Property;
    let isCallable = false;

    switch (entry.kind) {
        case 'plugin':
        case 'module':
            kind = vscode.CompletionItemKind.Module;
            break;
        case 'keyword':
            kind = vscode.CompletionItemKind.Keyword;
            break;
        case 'annotation':
            kind = vscode.CompletionItemKind.Keyword;
            break;
        case 'struct':
        case 'class':
            kind = vscode.CompletionItemKind.Struct;
            isCallable = false;
            break;
        case 'property':
            kind = vscode.CompletionItemKind.Property;
            isCallable = false;
            break;
        case 'variable':
            kind = vscode.CompletionItemKind.Variable;
            isCallable = false;
            break;
        case 'method':
            kind = vscode.CompletionItemKind.Method;
            isCallable = true;
            break;
        case 'function':
            kind = vscode.CompletionItemKind.Function;
            isCallable = true;
            break;
        default:
            kind = vscode.CompletionItemKind.Property;
            if (entry.detail && (entry.detail.includes('fn ') || entry.detail.includes('->') || entry.detail.includes('method') || entry.detail.includes('function'))) {
                isCallable = true;
            }
            break;
    }

    const item = new vscode.CompletionItem(entry.label, kind);
    item.detail = entry.detail || '';
    if (entry.documentation) {
        const md = new vscode.MarkdownString(removeTableBorders(entry.documentation));
        md.supportHtml = true;
        item.documentation = md;
    }
    if (entry.sortText) {
        item.sortText = entry.sortText;
    }

    // Fix annotation @@ issue by stripping @ from insertText
    if (entry.kind === 'annotation' && entry.label.startsWith('@')) {
        item.insertText = entry.label.substring(1);
    } else if (isCallable && !hasTrailingParen) {
        // Functions and methods are callable with parameter hints
        item.insertText = new vscode.SnippetString(`${entry.label}($0)`);
        item.command = { command: 'editor.action.triggerParameterHints', title: 'Trigger Parameter Hints' };
    } else {
        item.insertText = entry.label;
        if (isCallable) {
            item.command = { command: 'editor.action.triggerParameterHints', title: 'Trigger Parameter Hints' };
        }
    }

    return item;
}

function activate(context) {
    const diagnostics = vscode.languages.createDiagnosticCollection('flame');
    context.subscriptions.push(diagnostics);

    async function refreshDiagnostics(document) {
        const supportedLanguages = ['flame'];
        if (!document || !supportedLanguages.includes(document.languageId) || document.uri.fsPath.endsWith('.fmi') || document.uri.fsPath.endsWith('.tmp.fm')) return;
        const result = await runCheck(document);
        if (!result) {
            diagnostics.set(document.uri, []);
            return;
        }

        const mapped = (result.diagnostics || []).map((diag) => {
            const line = Math.max(0, (diag.line || 1) - 1);
            const col = Math.max(0, (diag.column || 1) - 1);
            const severity = diag.severity === 'warning'
                ? vscode.DiagnosticSeverity.Warning
                : diag.severity === 'info'
                    ? vscode.DiagnosticSeverity.Information
                    : vscode.DiagnosticSeverity.Error;
            return new vscode.Diagnostic(
                new vscode.Range(line, col, line, col + 1),
                diag.message,
                severity
            );
        });

        diagnostics.set(document.uri, mapped);
    }

    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(refreshDiagnostics),
        vscode.workspace.onDidSaveTextDocument(refreshDiagnostics),
        vscode.workspace.onDidChangeTextDocument((event) => refreshDiagnostics(event.document))
    );

    if (vscode.window.activeTextEditor) {
        refreshDiagnostics(vscode.window.activeTextEditor.document);
    }

    context.subscriptions.push(vscode.languages.registerCompletionItemProvider(['flame'], {
        async provideCompletionItems(document, position) {
            const result = await runCheck(document, position);
            if (!result) return [];
            const lineText = document.lineAt(position.line).text;
            const afterCursor = lineText.slice(position.character).trimStart();
            const hasTrailingParen = afterCursor.startsWith('(');
            return (result.completions || []).map(entry => toCompletionItem(entry, hasTrailingParen));
        }
    }, '.', '@', ':'));

    const tokenTypes = ['keyword', 'function', 'annotation', 'comment', 'string', 'enum', 'enumMember', 'type', 'variable', 'operator'];
    const tokenModifiers = ['declaration', 'readonly'];
    const legend = new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);

    function isIdentStart(ch) {
        return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_';
    }

    function isIdentPart(ch) {
        return isIdentStart(ch) || (ch >= '0' && ch <= '9');
    }

    context.subscriptions.push(
        vscode.languages.registerDocumentSemanticTokensProvider(['flame'], {
            async provideDocumentSemanticTokens(document) {
                const tokensBuilder = new vscode.SemanticTokensBuilder(legend);
                const result = await runCheck(document);
                if (result && result.tokens) {
                    for (const t of result.tokens) {
                        tokensBuilder.push(
                            t.line,
                            t.col,
                            t.length,
                            t.token_type,
                            t.token_modifiers
                        );
                    }
                }
                return tokensBuilder.build();
            }
        }, legend)
    );

    context.subscriptions.push(vscode.languages.registerHoverProvider(['flame'], {
        async provideHover(document, position) {
            const actionMd = getHoverActionButtons(document, position);
            const result = await runCheck(document, position);

            const blocks = [];
            if (actionMd) {
                blocks.push(actionMd);
            }

            if (result && result.hover) {
                if (result.hover.documentation) {
                    const md = new vscode.MarkdownString(removeTableBorders(result.hover.documentation));
                    md.supportHtml = true;
                    blocks.push(md);
                } else if (result.hover.label) {
                    blocks.push(new vscode.MarkdownString('`' + result.hover.label + '`'));
                }
            }

            if (blocks.length === 0) return null;
            return new vscode.Hover(blocks);
        }
    }));

    context.subscriptions.push(vscode.languages.registerDefinitionProvider(['flame'], {
        async provideDefinition(document, position) {
            const result = await runCheck(document, position);
            if (!result || !result.definition) return null;

            const targetUri = vscode.Uri.file(result.definition.file);
            const targetLine = Math.max(0, (result.definition.line || 1) - 1);
            const targetCol = Math.max(0, (result.definition.column || 1) - 1);
            const endLine = result.definition.end_line ? Math.max(0, result.definition.end_line - 1) : targetLine;
            const endCol = result.definition.end_column ? Math.max(0, result.definition.end_column - 1) : targetCol;

            return new vscode.Location(
                targetUri,
                new vscode.Range(targetLine, targetCol, endLine, endCol)
            );
        }
    }));

    context.subscriptions.push(vscode.languages.registerSignatureHelpProvider(['flame'], {
        async provideSignatureHelp(document, position) {
            const result = await runCheck(document, position);
            if (!result || !result.signature_help) {
                return null;
            }

            const signature = new vscode.SignatureInformation(
                result.signature_help.label
            );
            signature.parameters = result.signature_help.parameters.map(p => new vscode.ParameterInformation(p));

            const help = new vscode.SignatureHelp();
            help.signatures = [signature];
            help.activeSignature = 0;
            help.activeParameter = result.signature_help.active_parameter;

            return help;
        }
    }, '(', ','));

    context.subscriptions.push(vscode.languages.registerDocumentFormattingEditProvider(['flame'], {
        async provideDocumentFormattingEdits(document) {
            if (document.uri.fsPath.endsWith('.fmi') || document.uri.fsPath.endsWith('.tmp.fm')) return [];
            const workspaceRoot = findWorkspaceRoot(document.uri.fsPath);

            const args = ['format', document.uri.fsPath, '--stdout', '--stdin'];
            const compiler = findCompilerBinary(workspaceRoot);
            if (!compiler) return [];

            return new Promise((resolve) => {
                let options = { cwd: workspaceRoot, maxBuffer: 10 * 1024 * 1024 };
                if (process.platform === 'win32' || !compiler.includes('\\') && !compiler.includes('/')) {
                    options.shell = true;
                }
                const child = execFile(compiler, args, options, (error, stdout) => {
                    if (error) {
                        resolve([]);
                        return;
                    }

                    if (stdout) {
                        const fullRange = new vscode.Range(
                            document.positionAt(0),
                            document.positionAt(document.getText().length)
                        );
                        resolve([vscode.TextEdit.replace(fullRange, stdout)]);
                    } else {
                        resolve([]);
                    }
                });

                child.stdin.write(document.getText());
                child.stdin.end();
            });
        }
    }));

    context.subscriptions.push(vscode.languages.registerFoldingRangeProvider(['flame', 'flame-interface'], {
        provideFoldingRanges(document) {
            return computeFoldingRanges(document);
        }
    }));

    function getOrCreateTerminal(name = 'Flame') {
        let terminal = vscode.window.terminals.find(t => t.name === name);
        if (!terminal) {
            terminal = vscode.window.createTerminal(name);
        }
        return terminal;
    }

    function runInTerminal(cmd) {
        const terminal = getOrCreateTerminal('Flame');
        terminal.show(true);
        terminal.sendText(cmd);
    }

    context.subscriptions.push(
        vscode.commands.registerCommand('flame.runTest', (filePath, testName) => {
            const projectDir = findProjectDir(filePath);
            const relPath = filePath ? relative(projectDir, filePath) : '';
            const filterArg = testName ? ` --filter ${testName}` : '';
            const testTarget = relPath ? ` "${relPath}"` : '';
            runInTerminal(`cd "${projectDir}" && fmp test${testTarget}${filterArg}`);
        }),
        vscode.commands.registerCommand('flame.debugTest', (filePath, testName) => {
            const projectDir = findProjectDir(filePath);
            const relPath = filePath ? relative(projectDir, filePath) : '';
            const filterArg = testName ? ` --filter ${testName}` : '';
            const testTarget = relPath ? ` "${relPath}"` : '';
            runInTerminal(`cd "${projectDir}" && fmp test${testTarget}${filterArg} --debug`);
        }),
        vscode.commands.registerCommand('flame.runApp', (filePath) => {
            const projectDir = findProjectDir(filePath);
            const relPath = filePath ? relative(projectDir, filePath) : '';
            const isDefaultMain = !relPath || relPath === 'src/main.fm' || relPath === 'main.fm';
            const runCmd = isDefaultMain ? 'fmp run' : `fmp run "${relPath}"`;
            runInTerminal(`cd "${projectDir}" && ${runCmd}`);
        }),
        vscode.commands.registerCommand('flame.debugApp', (filePath) => {
            const projectDir = findProjectDir(filePath);
            const relPath = filePath ? relative(projectDir, filePath) : '';
            const isDefaultMain = !relPath || relPath === 'src/main.fm' || relPath === 'main.fm';
            const runCmd = isDefaultMain ? 'fmp run --debug' : `fmp run "${relPath}" --debug`;
            runInTerminal(`cd "${projectDir}" && ${runCmd}`);
        }),
        vscode.languages.registerCodeLensProvider(['flame'], {
            provideCodeLenses(document) {
                return getCodeLenses(document);
            }
        })
    );
}

const TEST_ANNO_RE = /^\s*@(Test|test|ExpectPanic|expect_panic|Benchmark|benchmark)\b/;
const APP_ANNO_RE = /^\s*@(Application|application|Cli|cli)\b/;
const FUNC_DECL_RE = /^\s*(?:pub\s+)?(?:export\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)/;

function findAnnotatedFunctionBelow(lines, startLine) {
    for (let i = startLine + 1; i < lines.length && i <= startLine + 10; i++) {
        const line = lines[i].trim();
        if (!line || line.startsWith('//') || line.startsWith('/*')) continue;
        const match = lines[i].match(FUNC_DECL_RE);
        if (match) {
            return { name: match[1], line: i };
        }
        if (line.startsWith('@')) continue;
        break;
    }
    return null;
}

function findAnnotationsAbove(lines, startLine) {
    const annos = [];
    for (let i = startLine - 1; i >= 0 && i >= startLine - 10; i--) {
        const line = lines[i].trim();
        if (!line || line.startsWith('//') || line.startsWith('/*')) continue;
        if (line.startsWith('@')) {
            annos.push({ text: line, line: i });
        } else {
            break;
        }
    }
    return annos;
}

function createTestActionButtons(filePath, testName) {
    const runUri = vscode.Uri.parse(`command:flame.runTest?${encodeURIComponent(JSON.stringify([filePath, testName]))}`);
    const debugUri = vscode.Uri.parse(`command:flame.debugTest?${encodeURIComponent(JSON.stringify([filePath, testName]))}`);
    const md = new vscode.MarkdownString(`[▶︎ Run Test](${runUri}) &nbsp;|&nbsp; [⚙︎ Debug](${debugUri})`);
    md.isTrusted = true;
    md.supportHtml = true;
    return md;
}

function createAppActionButtons(filePath) {
    const runUri = vscode.Uri.parse(`command:flame.runApp?${encodeURIComponent(JSON.stringify([filePath]))}`);
    const debugUri = vscode.Uri.parse(`command:flame.debugApp?${encodeURIComponent(JSON.stringify([filePath]))}`);
    const md = new vscode.MarkdownString(`[▶︎ Run](${runUri}) &nbsp;|&nbsp; [⚙︎ Debug](${debugUri})`);
    md.isTrusted = true;
    md.supportHtml = true;
    return md;
}

function getHoverActionButtons(document, position) {
    if (!document || document.uri.fsPath.endsWith('.fmi') || document.uri.fsPath.endsWith('.tmp.fm')) return null;
    const lines = document.getText().split('\n');
    const currentLineIndex = position.line;
    if (currentLineIndex < 0 || currentLineIndex >= lines.length) return null;
    const currentLine = lines[currentLineIndex];

    // 1. Cursor is on @Test / @ExpectPanic line
    if (TEST_ANNO_RE.test(currentLine)) {
        const func = findAnnotatedFunctionBelow(lines, currentLineIndex);
        if (func) {
            return createTestActionButtons(document.uri.fsPath, func.name);
        }
    }

    // 2. Cursor is on @Application / @Cli line
    if (APP_ANNO_RE.test(currentLine)) {
        return createAppActionButtons(document.uri.fsPath);
    }

    // 3. Cursor is on a function declaration line
    const funcMatch = currentLine.match(FUNC_DECL_RE);
    if (funcMatch) {
        const funcName = funcMatch[1];
        const annosAbove = findAnnotationsAbove(lines, currentLineIndex);
        if (annosAbove.some(a => TEST_ANNO_RE.test(a.text))) {
            return createTestActionButtons(document.uri.fsPath, funcName);
        }
        if (annosAbove.some(a => APP_ANNO_RE.test(a.text))) {
            return createAppActionButtons(document.uri.fsPath);
        }
    }

    return null;
}

function getCodeLenses(document) {
    if (!document || document.uri.fsPath.endsWith('.fmi') || document.uri.fsPath.endsWith('.tmp.fm')) return [];
    const codeLenses = [];
    const lines = document.getText().split('\n');
    const seenFunctions = new Set();

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (TEST_ANNO_RE.test(line)) {
            const func = findAnnotatedFunctionBelow(lines, i);
            if (func && !seenFunctions.has(func.line)) {
                seenFunctions.add(func.line);
                const range = new vscode.Range(i, 0, i, 0);
                codeLenses.push(
                    new vscode.CodeLens(range, {
                        title: '▶︎ Run Test',
                        command: 'flame.runTest',
                        arguments: [document.uri.fsPath, func.name]
                    }),
                    new vscode.CodeLens(range, {
                        title: '⚙︎ Debug',
                        command: 'flame.debugTest',
                        arguments: [document.uri.fsPath, func.name]
                    })
                );
            }
        } else if (APP_ANNO_RE.test(line)) {
            const range = new vscode.Range(i, 0, i, 0);
            codeLenses.push(
                new vscode.CodeLens(range, {
                    title: '▶︎ Run',
                    command: 'flame.runApp',
                    arguments: [document.uri.fsPath]
                }),
                new vscode.CodeLens(range, {
                    title: '⚙︎ Debug',
                    command: 'flame.debugApp',
                    arguments: [document.uri.fsPath]
                })
            );
        }
    }
    return codeLenses;
}

function computeFoldingRanges(document) {
    const ranges = [];
    const text = document.getText();
    const lines = text.split('\n');

    let inBlockComment = false;
    let commentStartLine = 0;

    let inMultilineTripleString = false;
    let inStringQuote = null; // '"' or "'"

    const braceStack = [];
    const bracketStack = [];
    const parenStack = [];

    let currentAnnotation = null; // { startLine: number, depth: number }

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
        const line = lines[lineIndex];
        const trimmed = line.trim();

        // Check if an annotation starts on this line (e.g. @Docs(...), @Flamer(...), @Suggestion(...))
        if (!inBlockComment && inStringQuote === null && !inMultilineTripleString) {
            const annoMatch = trimmed.match(/^@([a-zA-Z0-9_]+)\s*\(/);
            if (annoMatch && currentAnnotation === null) {
                currentAnnotation = {
                    startLine: lineIndex,
                    depth: 0
                };
            }
        }

        let i = 0;
        while (i < line.length) {
            // 1. Block comment continuation
            if (inBlockComment) {
                if (line.substr(i, 2) === '*/') {
                    inBlockComment = false;
                    if (lineIndex > commentStartLine) {
                        ranges.push(new vscode.FoldingRange(commentStartLine, lineIndex, vscode.FoldingRangeKind.Comment));
                    }
                    i += 2;
                    continue;
                }
                i++;
                continue;
            }

            // 2. Triple quote string continuation ("""...""")
            if (inMultilineTripleString) {
                if (line.substr(i, 3) === '"""') {
                    inMultilineTripleString = false;
                    i += 3;
                    continue;
                }
                i++;
                continue;
            }

            // 3. Regular string continuation across lines (like @Docs("...\n..."))
            if (inStringQuote !== null) {
                if (line[i] === '\\') {
                    i += 2;
                    continue;
                }
                if (line[i] === inStringQuote) {
                    inStringQuote = null;
                    i++;
                    continue;
                }
                i++;
                continue;
            }

            // Check start of block comment
            if (line.substr(i, 2) === '/*') {
                inBlockComment = true;
                commentStartLine = lineIndex;
                i += 2;
                continue;
            }

            // Check line comment
            if (line.substr(i, 2) === '//') {
                break;
            }

            // Check triple quote string start
            if (line.substr(i, 3) === '"""' || (line[i] === '$' && line.substr(i + 1, 3) === '"""')) {
                inMultilineTripleString = true;
                i += (line[i] === '$' ? 4 : 3);
                continue;
            }

            // Check regular string start (single or double quote)
            if (line[i] === '"' || line[i] === '\'') {
                inStringQuote = line[i];
                i++;
                continue;
            }

            const char = line[i];

            // Annotation tracking
            if (currentAnnotation !== null) {
                if (char === '(') {
                    currentAnnotation.depth++;
                } else if (char === ')') {
                    currentAnnotation.depth--;
                    if (currentAnnotation.depth <= 0) {
                        // "keep the annoations closables only when it more than 2 lines if not then as it is no line closeble bullet"
                        if (lineIndex - currentAnnotation.startLine >= 2) {
                            ranges.push(new vscode.FoldingRange(currentAnnotation.startLine, lineIndex));
                        }
                        currentAnnotation = null;
                    }
                }
            }

            // Braces: functions, closures, formula objects, structs, blocks
            if (char === '{') {
                braceStack.push(lineIndex);
            } else if (char === '}') {
                if (braceStack.length > 0) {
                    const start = braceStack.pop();
                    if (lineIndex > start) {
                        ranges.push(new vscode.FoldingRange(start, lineIndex));
                    }
                }
            } else if (char === '[') {
                bracketStack.push(lineIndex);
            } else if (char === ']') {
                if (bracketStack.length > 0) {
                    const start = bracketStack.pop();
                    if (lineIndex > start) {
                        ranges.push(new vscode.FoldingRange(start, lineIndex));
                    }
                }
            } else if (char === '(' && currentAnnotation === null) {
                parenStack.push(lineIndex);
            } else if (char === ')' && currentAnnotation === null) {
                if (parenStack.length > 0) {
                    const start = parenStack.pop();
                    if (lineIndex - start >= 2) {
                        ranges.push(new vscode.FoldingRange(start, lineIndex));
                    }
                }
            }

            i++;
        }
    }

    return ranges;
}

function deactivate() { }

module.exports = {
    activate,
    deactivate,
};