import React, { useState, useEffect, useRef } from 'react';
import Editor from '@monaco-editor/react';
import { Play, RotateCcw, Copy, Check, Terminal, Sparkles, CheckCircle2, AlertCircle, Clock } from 'lucide-react';
import { STD_DEFINITIONS, STD_HOVERS } from './stdData';

const PRESETS: Record<string, { name: string; code: string }> = {
  hello: {
    name: 'Hello Flame',
    code: `// Welcome to the Flame Interactive Playground!
// The Blaze VM runs directly in your browser via WebAssembly.

let name = "Developer"
println($"🔥 Hello, {name}! Welcome to Flame.")
    
let mut count = 0
for i in 1..=5 {
  count += i
}
println($"Sum of 1 to 5 is: {count}")

`
  },
  fibonacci: {
    name: 'Fibonacci & Recursion',
    code: `fn fib(n: Int) -> Int {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

println("--- Fibonacci Sequence ---")
for i in 0..10 {
    println($"fib({i}) = {fib(i)}")
}

`
  },
  structs: {
    name: 'Structs & Impl',
    code: `struct Player {
    name: String,
    level: Int,
    hp: Float,
}

impl Player {
    fn new(name: String) -> Player {
        return Player {
            name: name,
            level: 1,
            hp: 100.0,
        }
    }

    fn take_damage(&mut self, amount: Float) {
        self.hp = self.hp - amount
        println($"{self.name} took {amount} damage! HP: {self.hp}")
    }

    fn level_up(&mut self) {
        self.level = self.level + 1
        self.hp = self.hp + 20.0
        println($"⭐ {self.name} leveled up to Level {self.level}!")
    }
}

let mut hero = Player.new("Ignis")
println($"Created hero: {hero.name} [Lvl {hero.level}, HP {hero.hp}]")
hero.take_damage(25.5)
hero.level_up()
`
  },
  json_collections: {
    name: 'JSON & Data Processing',
    code: `import std.json

let raw = "{\\"service\\": \\"flame-cloud\\", \\"instances\\": 8, \\"ready\\": true}"
let parsed = json.parse(raw)
    
println($"Service: {parsed.service}")
println($"Active instances: {parsed.instances}")
println($"Service ready: {parsed.ready}")
    
let serialized = json.stringify(parsed)
println($"Serialized back: {serialized}")

`
  },
  math: {
    name: 'Math & Trigonometry',
    code: `import std.math

let pi = math.pi
let angle_deg = 45.0
let angle_rad = angle_deg * (pi / 180.0)
    
println($"Angle: {angle_deg} degrees")
println($"sin(45°): {math.sin(angle_rad)}")
println($"cos(45°): {math.cos(angle_rad)}")
println($"Square root of 256: {math.sqrt(256.0)}")
println($"2^10 = {math.pow(2.0, 10.0)}")

`
  }
};

export default function FlamePlayground() {
  const [selectedPreset, setSelectedPreset] = useState<string>('hello');
  const [code, setCode] = useState<string>(PRESETS.hello.code);
  const [output, setOutput] = useState<string>('Click "Run Code" or press Ctrl+Enter to execute.\n');
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [wasmReady, setWasmReady] = useState<boolean>(false);
  const [statusMessage, setStatusMessage] = useState<string>('Initializing WebAssembly runtime...');
  const [executionTime, setExecutionTime] = useState<number | null>(null);
  const [isSuccess, setIsSuccess] = useState<boolean | null>(null);
  const [copied, setCopied] = useState<boolean>(false);

  const wasmInstanceRef = useRef<WebAssembly.Instance | null>(null);
  const editorRef = useRef<any>(null);

  // Load WASM Runtime
  useEffect(() => {
    let isMounted = true;

    async function loadWasm() {
      try {
        setStatusMessage('Loading Flame WebAssembly runtime...');
        const response = await fetch('/flame_runtime.wasm');
        if (!response.ok) {
          throw new Error('Failed to fetch flame_runtime.wasm: ' + response.statusText);
        }
        const bytes = await response.arrayBuffer();
        const { instance } = await WebAssembly.instantiate(bytes, {});

        if (isMounted) {
          wasmInstanceRef.current = instance;
          setWasmReady(true);
          setStatusMessage('Flame WASM runtime active (Ready)');
        }
      } catch (err: any) {
        console.error('Failed to initialize Flame WASM:', err);
        if (isMounted) {
          setStatusMessage('Failed to load WASM (' + err.message + ')');
        }
      }
    }

    loadWasm();
    return () => {
      isMounted = false;
    };
  }, []);

  // Configure Monaco Editor for Flame
  function handleEditorWillMount(monaco: any) {
    if (monaco.languages.getLanguages().some((l: any) => l.id === 'flame')) {
      return;
    }

    monaco.languages.register({ id: 'flame', extensions: ['.fm'] });

    // Language configuration (brackets, comments, pairs)
    monaco.languages.setLanguageConfiguration('flame', {
      comments: {
        lineComment: '//',
        blockComment: ['/*', '*/'],
      },
      brackets: [
        ['{', '}'],
        ['[', ']'],
        ['(', ')'],
      ],
      autoClosingPairs: [
        { open: '{', close: '}' },
        { open: '[', close: ']' },
        { open: '(', close: ')' },
        { open: '"', close: '"', notIn: ['string'] },
        { open: '`', close: '`', notIn: ['string'] },
      ],
      surroundingPairs: [
        { open: '{', close: '}' },
        { open: '[', close: ']' },
        { open: '(', close: ')' },
        { open: '"', close: '"' },
        { open: '`', close: '`' },
      ],
    });

    // Monarch Syntax Tokenizer matching Flame grammar
    monaco.languages.setMonarchTokensProvider('flame', {
      defaultToken: 'invalid',
      keywords: [
        'let', 'const', 'mut', 'fn', 'struct', 'enum', 'trait', 'impl', 'if', 'else',
        'match', 'for', 'in', 'while', 'loop', 'break', 'continue', 'return', 'yield',
        'await', 'async', 'thread', 'package', 'export', 'import', 'plugin', 'type',
        'where', 'formula', 'annotation', 'as', 'and', 'or', 'not', 'defer'
      ],
      typeKeywords: [
        'Int', 'Num', 'Float', 'String', 'Bool', 'Array', 'Map', 'Option', 'Result',
        'Vector', 'Void', 'Any', 'Byte', 'Char', 'Never', 'Self', 'Nil'
      ],
      constants: ['true', 'false', 'nil', 'self'],
      operators: [
        '=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=',
        '==', '!=', '===', '!==', '<', '<=', '>', '>=',
        '+', '++', '-', '--', '*', '/', '%',
        '&', '|', '^', '~', '<<', '>>',
        '&&', '||', '!', '?.', '?:',
        '->', '=>', '|>', '..', '..=', '...'
      ],
      tokenizer: {
        root: [
          // Comments
          [/\/\/.*$/, 'comment'],
          [/\/\*/, 'comment', '@comment'],

          // Annotations (@Application, @Test, @Benchmark, etc.) - KEYWORD RED
          [/@[a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*/, 'annotation'],

          // String Interpolation: $"..."
          [/\$"/, { token: 'string.quote', next: '@interpolated_string' }],

          // Regular Double-Quoted Strings
          [/"/, { token: 'string.quote', next: '@string' }],

          // Single-quoted & Backtick Strings
          [/'[^\\']*'/, 'string'],
          [/`[^\\`]*`/, 'string'],

          // Numbers
          [/0x[0-9a-fA-F]+/, 'number.hex'],
          [/\d+\.\d+/, 'number.float'],
          [/\d+/, 'number'],

          // Identifiers & Keywords
          [/[a-zA-Z_][a-zA-Z0-9_]*/, {
            cases: {
              '@keywords': 'keyword',
              '@typeKeywords': 'type',
              '@constants': 'constant',
              '@default': 'identifier',
            }
          }],

          // Delimiters and operators
          [/[{}()\[\]]/, '@brackets'],
          [/->|=>|\|>|\.\.=?/, 'operator'],
          [/[=><!~?:&|+\-*\/\^%]+/, {
            cases: {
              '@operators': 'operator',
              '@default': 'operator'
            }
          }],
        ],

        interpolated_string: [
          [/[^"\\{]+/, 'string'],
          [/\\./, 'string.escape'],
          [/\{/, { token: 'delimiter.bracket', next: '@interpolated_expression' }],
          [/"/, { token: 'string.quote', next: '@pop' }]
        ],

        interpolated_expression: [
          [/\{/, { token: 'delimiter.bracket', next: '@push' }],
          [/\}/, { token: 'delimiter.bracket', next: '@pop' }],
          { include: 'root' }
        ],

        string: [
          [/[^"\\]+/, 'string'],
          [/\\./, 'string.escape'],
          [/"/, { token: 'string.quote', next: '@pop' }]
        ],

        comment: [
          [/[^\/*]+/, 'comment'],
          [/\*\//, 'comment', '@pop'],
          [/[\/*]/, 'comment']
        ],
      }
    });

    // Autocomplete Suggestions from ide/ and Blaze/std
    monaco.languages.registerCompletionItemProvider('flame', {
      triggerCharacters: ['.', '@', ':', '$'],
      provideCompletionItems: (model: any, position: any) => {
        const word = model.getWordUntilPosition(position);
        const lineContent = model.getLineContent(position.lineNumber);
        const textBefore = lineContent.substring(0, position.column - 1);

        const normalRange = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };

        // 1. Check if typing an annotation starting with @
        const atMatch = textBefore.match(/@([a-zA-Z0-9_]*)$/);
        if (atMatch !== null) {
          const annotationRange = {
            startLineNumber: position.lineNumber,
            endLineNumber: position.lineNumber,
            startColumn: position.column - atMatch[0].length,
            endColumn: position.column,
          };

          return {
            suggestions: [
              {
                label: '@Application',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Application',
                detail: 'Application Entry Point',
                documentation: 'Marks this function as the application entry point invoked automatically at startup.',
                range: annotationRange,
              },
              {
                label: '@Test',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Test',
                detail: 'Unit Test Case',
                documentation: 'Marks this function as a test case executed by the Flame test runner.',
                range: annotationRange,
              },
              {
                label: '@Benchmark',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Benchmark',
                detail: 'Performance Benchmark',
                documentation: 'Executes the function as a high-precision performance benchmark with statistics.',
                range: annotationRange,
              },
              {
                label: '@Cli',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Cli',
                detail: 'CLI Tool Marker',
                documentation: 'Marks the application as a Command Line Interface tool with argument parsing.',
                range: annotationRange,
              },
              {
                label: '@Command',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Command',
                detail: 'CLI Subcommand',
                documentation: 'Registers a function as an executable subcommand in a @Cli application.',
                range: annotationRange,
              },
              {
                label: '@Requires',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Requires("${1:module}")',
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                detail: 'Scoped Module Requirement',
                documentation: 'Injects a dependency into function scope without globally importing it.',
                range: annotationRange,
              },
              {
                label: '@Permission',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Permission("${1:net}")',
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                detail: 'Access Permission Request',
                documentation: 'Declares required runtime permissions (net, fs, env, os).',
                range: annotationRange,
              },
              {
                label: '@Platform',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Platform("${1:linux}")',
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                detail: 'Conditional Target Compilation',
                documentation: 'Conditionally compiles declarations only for matching OS platforms.',
                range: annotationRange,
              },
              {
                label: '@Docs',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Docs("${1:documentation}")',
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                detail: 'IDE Documentation Metadata',
                documentation: 'Provides rich hover markdown documentation for items.',
                range: annotationRange,
              },
              {
                label: '@Setup',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Setup',
                detail: 'Test Fixture Setup',
                documentation: 'Runs before each test case in the module.',
                range: annotationRange,
              },
              {
                label: '@Cleanup',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@Cleanup',
                detail: 'Test Fixture Teardown',
                documentation: 'Runs after each test case in the module.',
                range: annotationRange,
              },
              {
                label: '@ExpectPanic',
                kind: monaco.languages.CompletionItemKind.Keyword,
                insertText: '@ExpectPanic',
                detail: 'Expected Failure Test',
                documentation: 'Asserts that the test function must terminate with panic or error.',
                range: annotationRange,
              },
            ]
          };
        }

        // 2. Check if typing after 'std.'
        const stdMatch = textBefore.match(/\bstd\.\s*([a-zA-Z0-9_]*)$/);
        if (stdMatch !== null) {
          const stdModules = [
            { name: 'math', desc: 'Trigonometry, roots, powers, logarithms, and constants' },
            { name: 'json', desc: 'JSON parser and serializer (parse, stringify, pretty)' },
            { name: 'fs', desc: 'File system operations (read, write, append, dir)' },
            { name: 'net', desc: 'Networking, TCP/UDP sockets, HTTP client' },
            { name: 'time', desc: 'Timers, benchmarks, epoch timestamps, duration' },
            { name: 'os', desc: 'OS info, command-line arguments, process PID, exit' },
            { name: 'env', desc: 'System environment variables (get, set, has, all)' },
            { name: 'desktop', desc: 'Desktop launcher, URLs, custom schemes, apps' },
            { name: 'window', desc: 'Desktop window manager (find, focus, minimize, resize)' },
            { name: 'web', desc: 'Browser DOM manipulation, dialogs (alert, prompt, confirm)' },
            { name: 'byte', desc: 'Binary byte buffers, hex conversion, endian streams' },
            { name: 'camera', desc: 'Hardware camera capture and device list' },
            { name: 'thread', desc: 'Concurrency, thread spawning, background workers' },
          ];

          return {
            suggestions: stdModules.map((m) => ({
              label: m.name,
              kind: monaco.languages.CompletionItemKind.Module,
              insertText: m.name,
              detail: `std.${m.name}`,
              documentation: m.desc,
              range: normalRange,
            }))
          };
        }

        // 3. Check if typing after a module dot (e.g. 'math.', 'json.', 'fs.', 'std.math.')
        const dotMatch = textBefore.match(/(?:(?:std\.)?([a-zA-Z0-9_]+))\s*\.\s*([a-zA-Z0-9_]*)$/);
        if (dotMatch !== null) {
          const modName = dotMatch[1];
          const modItems = STD_DEFINITIONS.filter((item) => item.module === modName);
          if (modItems.length > 0) {
            return {
              suggestions: modItems.map((item) => {
                const prefix = item.module + '.';
                const snippet = item.insertText.startsWith(prefix) ? item.insertText.substring(prefix.length) : item.insertText;
                return {
                  label: item.name,
                  kind: monaco.languages.CompletionItemKind.Function,
                  insertText: snippet,
                  insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                  detail: item.detail,
                  documentation: { value: item.documentation },
                  range: normalRange,
                };
              })
            };
          }
        }

        // 4. Default / Top-level Completions: Keywords, Builtins, Modules, and Std Functions
        const suggestions = [
          // Keywords & Snippets
          {
            label: 'fn',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['fn ${1:name}(${2:params}) -> ${3:Void} {', '\t${0}', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Function Declaration',
            documentation: 'Declares a new statically typed Flame function.',
            range: normalRange,
          },
          {
            label: 'struct',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['struct ${1:Name} {', '\t${2:field}: ${3:Type},', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Struct Definition',
            documentation: 'Defines a custom structured product data type in Flame.',
            range: normalRange,
          },
          {
            label: 'impl',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['impl ${1:StructName} {', '\tfn new(${2:params}) -> ${1:StructName} {', '\t\t${0}', '\t}', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Implementation Block',
            documentation: 'Implements methods and associated constructors for a struct or enum.',
            range: normalRange,
          },
          {
            label: 'let mut',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: 'let mut ${1:name} = ${2:value}',
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Mutable Variable',
            documentation: 'Declares a mutable local variable.',
            range: normalRange,
          },
          {
            label: 'let',
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: 'let ${1:name} = ${2:value}',
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Immutable Variable',
            documentation: 'Declares an immutable local variable binding.',
            range: normalRange,
          },
          {
            label: 'const',
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: 'const ${1:NAME}: ${2:Type} = ${3:value}',
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Constant Declaration',
            documentation: 'Declares an immutable compile-time constant.',
            range: normalRange,
          },
          {
            label: 'for in',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['for ${1:item} in ${2:collection} {', '\t${0}', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'For Loop',
            documentation: 'Iterates through ranges (e.g. 0..10) or collections.',
            range: normalRange,
          },
          {
            label: 'while',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['while ${1:condition} {', '\t${0}', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'While Loop',
            documentation: 'Loops while condition remains true.',
            range: normalRange,
          },
          {
            label: 'if else',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['if ${1:condition} {', '\t${2}', '} else {', '\t${0}', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Conditional Branch',
            documentation: 'Conditional if-else control flow statement.',
            range: normalRange,
          },
          {
            label: 'match',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['match ${1:expr} {', '\t${2:pattern} => ${3:result},', '\t_ => ${0},', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Pattern Match',
            documentation: 'Matches expressions against structural patterns.',
            range: normalRange,
          },
          {
            label: 'thread',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: ['thread {', '\t${0}', '}'].join('\n'),
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Spawn Thread',
            documentation: 'Spawns a native concurrent execution thread.',
            range: normalRange,
          },
          {
            label: 'import',
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: 'import std.${1:module}',
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Module Import',
            documentation: 'Imports an external or standard library module.',
            range: normalRange,
          },
          {
            label: 'return',
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: 'return ${1:value}',
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: 'Return Value',
            documentation: 'Returns a value from a function.',
            range: normalRange,
          },

          // Built-in functions (directly available unqualified)
          ...STD_DEFINITIONS.filter((item) => item.module === 'builtins').map((item) => {
            const prefix = 'builtins.';
            const snippet = item.insertText.startsWith(prefix) ? item.insertText.substring(prefix.length) : item.insertText;
            return {
              label: item.name,
              kind: monaco.languages.CompletionItemKind.Function,
              insertText: snippet,
              insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
              detail: item.detail.replace('builtins.', ''),
              documentation: { value: item.documentation },
              range: normalRange,
            };
          }),

          // Standard Library Modules
          ...['math', 'json', 'fs', 'net', 'time', 'os', 'env', 'desktop', 'window', 'web', 'byte', 'camera', 'thread'].map((mod) => ({
            label: `std.${mod}`,
            kind: monaco.languages.CompletionItemKind.Module,
            insertText: `std.${mod}`,
            detail: `Standard Library: std.${mod}`,
            documentation: `Import or access standard module std.${mod}.`,
            range: normalRange,
          })),
          ...['math', 'json', 'fs', 'net', 'time', 'os', 'env', 'desktop', 'window', 'web', 'byte', 'camera', 'thread'].map((mod) => ({
            label: mod,
            kind: monaco.languages.CompletionItemKind.Module,
            insertText: mod,
            detail: `Module: ${mod}`,
            documentation: `Access standard module ${mod}.`,
            range: normalRange,
          })),

          // All std functions with module qualifiers (e.g. math.sqrt(${1:x}), json.parse(${1:string}))
          ...STD_DEFINITIONS.filter((item) => item.module !== 'builtins' && item.module !== 'annotation').map((item) => ({
            label: item.label,
            kind: monaco.languages.CompletionItemKind.Function,
            insertText: item.insertText,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: item.detail,
            documentation: { value: item.documentation },
            range: normalRange,
          })),

          // Types
          ...['Int', 'Float', 'String', 'Bool', 'Array', 'Map', 'Option', 'Result', 'Vector', 'Void', 'Any', 'Byte', 'Bytes', 'Window', 'Element', 'Formula'].map((t) => ({
            label: t,
            kind: monaco.languages.CompletionItemKind.Class,
            insertText: t,
            detail: `Built-in Type: ${t}`,
            documentation: `Flame primitive/composite type ${t}.`,
            range: normalRange,
          })),
        ];

        return { suggestions };
      }
    });

    // Hover Tooltips Provider matching src/ide/keywords.rs and Blaze/std
    monaco.languages.registerHoverProvider('flame', {
      provideHover: (model: any, position: any) => {
        let word = model.getWordAtPosition(position);
        const lineContent = model.getLineContent(position.lineNumber);

        // Check if hovering directly on '@'
        if (!word) {
          if (lineContent.charAt(position.column - 1) === '@') {
            const nextWord = model.getWordAtPosition({ lineNumber: position.lineNumber, column: position.column + 1 });
            if (nextWord) {
              const doc = HOVERS['@' + nextWord.word] || HOVERS[nextWord.word];
              if (doc) return { contents: [{ value: doc }] };
            }
          }
          return null;
        }

        const textBefore = lineContent.substring(0, word.startColumn - 1);
        const dotMatch = textBefore.match(/([a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)*)\.$/);
        const prefix = dotMatch ? dotMatch[1] : '';
        const qualifiedName = prefix ? `${prefix}.${word.word}` : word.word;
        const shortQualified = prefix.startsWith('std.') ? `${prefix.substring(4)}.${word.word}` : qualifiedName;

        const HOVERS: Record<string, string> = {
          '@Application': '```flame\nannotation @Application(features: ["String"])\n```\n**Application Entry Point**\n\nMarks this function as the application\'s entry point. The function is invoked automatically when the program starts.',
          Application: '```flame\nannotation @Application(features: ["String"])\n```\n**Application Entry Point**\n\nMarks this function as the application\'s entry point. The function is invoked automatically when the program starts.',
          '@Test': '```flame\nannotation @Test(timeout: Int = 1000, skip: Bool = false)\n```\n**Unit Test**\n\nMarks this function as a test case executed by the Flame test harness.',
          Test: '```flame\nannotation @Test(timeout: Int = 1000, skip: Bool = false)\n```\n**Unit Test**\n\nMarks this function as a test case executed by the Flame test harness.',
          '@Benchmark': '```flame\nannotation @Benchmark(warmup: Int = 10, iterations: Int = 100)\n```\n**Performance Benchmark**\n\nMarks this function as a high-precision performance benchmark.',
          Benchmark: '```flame\nannotation @Benchmark(warmup: Int = 10, iterations: Int = 100)\n```\n**Performance Benchmark**\n\nMarks this function as a high-precision performance benchmark.',
          '@Cli': '```flame\nannotation @Cli\n```\n**CLI Application**\n\nMarks the application as a Command Line Interface tool with automated argument parsing.',
          Cli: '```flame\nannotation @Cli\n```\n**CLI Application**\n\nMarks the application as a Command Line Interface tool with automated argument parsing.',
          '@Command': '```flame\nannotation @Command\n```\n**CLI Command**\n\nRegisters a function as an executable subcommand in a @Cli application.',
          Command: '```flame\nannotation @Command\n```\n**CLI Command**\n\nRegisters a function as an executable subcommand in a @Cli application.',
          '@Requires': '```flame\nannotation @Requires(String...)\n```\n**Dependency Requirement**\n\nSpecifies system or module dependencies required by this function scope.',
          Requires: '```flame\nannotation @Requires(String...)\n```\n**Dependency Requirement**\n\nSpecifies system or module dependencies required by this function scope.',
          '@Permission': '```flame\nannotation @Permission(String...)\n```\n**Access Permission**\n\nRequests specific runtime permissions (e.g. "net", "fs", "env").',
          Permission: '```flame\nannotation @Permission(String...)\n```\n**Access Permission**\n\nRequests specific runtime permissions (e.g. "net", "fs", "env").',
          '@Platform': '```flame\nannotation @Platform(target: String)\n```\n**Conditional Compilation**\n\nConditionally compiles the annotated declaration for specific platforms.',
          Platform: '```flame\nannotation @Platform(target: String)\n```\n**Conditional Compilation**\n\nConditionally compiles the annotated declaration for specific platforms.',
          '@Docs': '```flame\nannotation @Docs(String...)\n```\n**Documentation Provider**\n\nProvides rich IDE hover documentation for items.',
          Docs: '```flame\nannotation @Docs(String...)\n```\n**Documentation Provider**\n\nProvides rich IDE hover documentation for items.',
          fn: '**fn** (Function Declaration)\n\nDeclares a statically typed function in Flame.\n\n```flame\nfn calculate(x: Int, y: Int) -> Int {\n    return x + y\n}\n```',
          struct: '**struct** (Structured Type)\n\nDefines a custom named product type with typed fields.\n\n```flame\nstruct User {\n    id: Int,\n    username: String,\n}\n```',
          impl: '**impl** (Method Implementation)\n\nImplements methods, constructors, and trait behaviors for a struct or enum.',
          let: '**let** (Local Binding)\n\nBinds an immutable local variable. Use `let mut` for mutable variables.',
          mut: '**mut** (Mutability Qualifier)\n\nMarks a variable or parameter as mutable.',
          const: '**const** (Constant Binding)\n\nDeclares an immutable compile-time constant.',
          println: '**println(values...) -> Nil**\n\nBuilt-in console output function. Evaluates and stringifies all arguments, appending a newline.',
          print: '**print(values...) -> Nil**\n\nPrints evaluated arguments directly to standard output without appending a newline.',
          eprint: '**eprint(values...) -> Nil**\n\nPrints values directly to standard error.',
          panic: '**panic(message: String) -> Never**\n\nTerminates execution immediately with an unrecoverable error and diagnostic trace.',
          import: '**import** (Module Import)\n\nImports external modules or standard libraries (e.g. `import std.json`, `import std.math`).',
          export: '**export** (Module Export)\n\nExports functions, structs, or annotations from a package.',
          package: '**package** (Package Declaration)\n\nDeclares the package namespace for the current file.',
          nil: '**nil** (Null Value)\n\nRepresents the absence of a value or uninitialized optional.',
          match: '**match** (Pattern Matching)\n\nExecutes pattern-based branch matching over enums, formulas, or literals.',
          thread: '**thread** (Concurrent Thread)\n\nSpawns a native concurrent background execution thread.',
          async: '**async** (Asynchronous Function)\n\nDeclares an asynchronous coroutine function.',
          await: '**await** (Await Future)\n\nSuspends until an asynchronous task or thread completes.',
          defer: '**defer** (Deferred Action)\n\nDefers statement execution until current scope exits.',
          while: '**while** (While Loop)\n\nExecutes statements repeatedly while a condition evaluates to true.',
          for: '**for** (For Loop)\n\nIterates over a sequence, range, or iterable collection.',
          in: '**in** (Collection Membership / Iteration Operator)\n\nUsed in `for item in collection` loops.',
          return: '**return** (Return Expression)\n\nReturns execution and an optional value from a function.',
          math: '**std.math** (Flame Mathematics Module)\n\nComprehensive mathematical utilities including trigonometry (`sin`, `cos`, `tan`), roots (`sqrt`), powers (`pow`), logarithms (`log`, `log2`, `log10`), and constants (`pi`, `e`, `tau`).',
          json: '**std.json** (JSON Serialization Module)\n\nHigh-performance JSON parser and serializer with support for dynamic formulas, objects, arrays, and pretty-printing (`parse`, `stringify`, `isValid`, `pretty`).',
          fs: '**std.fs** (File System Module)\n\nCross-platform file system operations for reading, writing, appending files, and traversing directories.',
          net: '**std.net** (Networking Module)\n\nTCP, UDP, HTTP client, and asynchronous socket communication streams.',
          desktop: '**std.desktop** (Desktop Integration Module)\n\nDesktop application launching, URL scheme dispatch (`https://`, `brave://`), and workspace process integration.',
          window: '**std.window** (Desktop Window Management Module)\n\nOperating system window manager interface: list active windows, minimize, maximize, restore, resize, focus, and close.',
          web: '**std.web** (Web and DOM Module)\n\nWebAssembly DOM bindings, element selection, dynamic modification, event listeners, and browser dialogs (`alert`, `prompt`, `confirm`).',
          byte: '**std.byte** (Binary and Buffer Module)\n\nByte buffers, hex conversions, endian-aware integer/float binary serialization, and file byte streaming.',
          time: '**std.time** (Time and Benchmarking Module)\n\nHigh-resolution monotonic time measurement, timestamps, durations, timers, and execution benchmarks.',
          os: '**std.os** (Operating System Utilities)\n\nProcess management, command-line arguments, architecture detection, environment queries, and exit codes.',
          env: '**std.env** (Environment Variables Module)\n\nAccess and mutate system environment variables (`get`, `set`, `has`, `all`).',
          camera: '**std.camera** (Camera Module)\n\nHardware camera device discovery, capture frames, and snapshot generation.',
          thread_mod: '**std.thread** (Threading and Concurrency Module)\n\nNative multithreading primitives, thread pools, background workers, and channels.',
          Int: '**Int**\n\n64-bit signed integer primitive type.',
          Float: '**Float**\n\n64-bit IEEE 754 floating point primitive type.',
          String: '**String**\n\nUTF-8 encoded heap-allocated string primitive type.',
          Bool: '**Bool**\n\nBoolean type representing either `true` or `false`.',
          Array: '**Array**\n\nHomogeneous dynamically sized array container.',
          Map: '**Map**\n\nKey-value hash map collection.',
          Formula: '**Formula**\n\nDynamic schema-flexible formula / record structure.',
          Bytes: '**Bytes**\n\nRaw binary byte array buffer with endian-aware numeric read/write methods.',
          Byte: '**Byte**\n\nSingle 8-bit unsigned byte value (0-255).',
          Window: '**Window**\n\nDesktop operating system window reference with properties like `title`, `appName`, `pid`, `bounds`.',
          Element: '**Element**\n\nWeb DOM element reference.',
        };

        const doc = STD_HOVERS[qualifiedName]
          || (shortQualified !== qualifiedName && STD_HOVERS[shortQualified])
          || STD_HOVERS[word.word]
          || HOVERS['@' + word.word]
          || HOVERS[word.word];

        if (doc) {
          return {
            contents: [{ value: doc }]
          };
        }
        return null;
      }
    });

    // Theme Definition matching GitHub Dark and docs code blocks
    monaco.editor.defineTheme('github-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [
        { token: 'keyword', foreground: 'ff7b72', fontStyle: 'bold' },
        { token: 'annotation', foreground: 'ff7b72', fontStyle: 'bold' },
        { token: 'type', foreground: 'ffa657' },
        { token: 'string', foreground: 'a5d6ff' },
        { token: 'string.quote', foreground: '79c0ff' },
        { token: 'string.escape', foreground: '79c0ff' },
        { token: 'delimiter.bracket', foreground: 'ff7b72', fontStyle: 'bold' },
        { token: 'number', foreground: '79c0ff' },
        { token: 'number.hex', foreground: '79c0ff' },
        { token: 'number.float', foreground: '79c0ff' },
        { token: 'comment', foreground: '8b949e', fontStyle: 'italic' },
        { token: 'operator', foreground: 'ff453a', fontStyle: 'bold' },
        { token: 'delimiter', foreground: 'c9d1d9' },
        { token: 'identifier', foreground: 'e6edf3' },
        { token: 'constant', foreground: '79c0ff' },
      ],
      colors: {
        'editor.background': '#000000',
        'editorGutter.background': '#000000',
        'editor.foreground': '#e6edf3',
        'editorLineNumber.foreground': '#484f58',
        'editorLineNumber.activeForeground': '#e6edf3',
        'editor.lineHighlightBackground': '#161b2255',
        'editor.selectionBackground': '#264f78',
        'editor.inactiveSelectionBackground': '#264f7844',
        'editorCursor.foreground': '#58a6ff',
        'editorWhitespace.foreground': '#484f58',
        'editorIndentGuide.background': '#21262d',
        'editorIndentGuide.activeBackground': '#30363d',
        'editorSuggestWidget.background': '#0d1117',
        'editorSuggestWidget.border': '#30363d',
        'editorSuggestWidget.foreground': '#c9d1d9',
        'editorSuggestWidget.selectedBackground': '#21262d',
        'editorSuggestWidget.highlightForeground': '#58a6ff',
        'editorHoverWidget.background': '#0d1117',
        'editorHoverWidget.border': '#30363d',
        'editorHoverWidget.foreground': '#c9d1d9',
      }
    });
  }

  // Execute Flame Code via WASM
  function runCode() {
    if (!wasmInstanceRef.current) {
      setOutput('Error: Flame WASM engine is not initialized yet. Please wait...');
      return;
    }

    setIsRunning(true);
    setIsSuccess(null);
    const startTime = performance.now();

    try {
      const wasm = wasmInstanceRef.current.exports as any;
      const encoder = new TextEncoder();
      const codeBytes = encoder.encode(code);

      // Allocate WASM memory and copy code
      const inputPtr = wasm.flame_alloc(codeBytes.length);
      new Uint8Array(wasm.memory.buffer, inputPtr, codeBytes.length).set(codeBytes);

      // Execute in Flame VM
      const resPtr = wasm.flame_eval(inputPtr, codeBytes.length);

      // Read length-prefixed response
      const view = new DataView(wasm.memory.buffer);
      const jsonLen = view.getUint32(resPtr, true);
      const jsonBytes = new Uint8Array(wasm.memory.buffer, resPtr + 4, jsonLen);
      const jsonStr = new TextDecoder().decode(jsonBytes);

      // Free allocated memory
      wasm.flame_free(inputPtr, codeBytes.length);
      wasm.flame_free(resPtr, 4 + jsonLen);

      const response = JSON.parse(jsonStr);
      const elapsed = performance.now() - startTime;
      setExecutionTime(Math.round(elapsed * 10) / 10);

      let displayText = '';
      if (response.output && response.output.trim().length > 0) {
        displayText += response.output + '\n';
      }
      if (response.result !== null && response.result !== undefined) {
        displayText += '=> ' + response.result + '\n';
      }

      if (response.success) {
        setIsSuccess(true);
        setOutput(displayText || '(Script completed successfully with no output)');
      } else {
        setIsSuccess(false);
        setOutput((displayText ? displayText + '\n' : '') + '❌ ' + (response.error || 'Execution failed'));
      }
    } catch (err: any) {
      console.error('Execution error:', err);
      setIsSuccess(false);
      setOutput('❌ Fatal Runtime Error: ' + err.message);
    } finally {
      setIsRunning(false);
    }
  }

  function handlePresetChange(presetKey: string) {
    setSelectedPreset(presetKey);
    if (PRESETS[presetKey]) {
      setCode(PRESETS[presetKey].code);
      setOutput('Loaded example: ' + PRESETS[presetKey].name + '. Click "Run Code" to execute.\n');
      setIsSuccess(null);
      setExecutionTime(null);
    }
  }

  function copyCode() {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  function resetCode() {
    if (PRESETS[selectedPreset]) {
      setCode(PRESETS[selectedPreset].code);
    }
  }

  return (
    <div
      className="flame-playground"
      style={{
        display: 'flex',
        flexDirection: 'column',
        borderRadius: '10px',
        overflow: 'hidden',
        border: '0.5px solid rgba(255, 255, 255, 0.1)',
        backgroundColor: '#000000',
        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.6)',
        margin: '24px 0',
        minHeight: '620px',
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace",
      }}
    >
      {/* Top Header / Toolbar */}
      <div style={{
        display: 'flex',
        flexWrap: 'wrap',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 16px',
        backgroundColor: '#000000',
        borderBottom: '0.5px solid rgba(255, 255, 255, 0.1)',
        gap: '12px',
      }}>
        {/* Left: Flame Title & Preset Selector */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '18px' }}>🔥</span>
            <span style={{ fontWeight: 600, color: '#ffffff', fontSize: '14px', letterSpacing: '-0.2px' }}>
              Flame Playground
            </span>
          </div>

          <select
            value={selectedPreset}
            onChange={(e) => handlePresetChange(e.target.value)}
            style={{
              backgroundColor: '#000000',
              color: '#e6edf3',
              border: '0.5px solid rgba(255, 255, 255, 0.18)',
              borderRadius: '6px',
              padding: '5px 10px',
              fontSize: '12px',
              fontWeight: 500,
              cursor: 'pointer',
              outline: 'none',
              fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, monospace",
            }}
          >
            {Object.entries(PRESETS).map(([key, item]) => (
              <option key={key} value={key} style={{ backgroundColor: '#0d1117', color: '#e6edf3' }}>
                {item.name}
              </option>
            ))}
          </select>
        </div>

        {/* Right: Actions */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          {/* Status Indicator */}
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            fontSize: '12px',
            color: wasmReady ? '#3fb950' : '#d29922',
            marginRight: '8px'
          }}>
            <span style={{
              width: '8px',
              height: '8px',
              borderRadius: '50%',
              backgroundColor: wasmReady ? '#3fb950' : '#d29922',
              display: 'inline-block',
              boxShadow: wasmReady ? '0 0 8px #3fb950' : 'none'
            }} />
            <span>{wasmReady ? 'WASM Ready' : 'Loading Engine...'}</span>
          </div>

          <button
            onClick={copyCode}
            title="Copy Code"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              backgroundColor: 'rgba(255, 255, 255, 0.05)',
              color: 'rgba(255, 255, 255, 0.8)',
              border: '0.5px solid rgba(255, 255, 255, 0.15)',
              borderRadius: '6px',
              padding: '6px 10px',
              fontSize: '12px',
              cursor: 'pointer',
              transition: 'all 0.2s',
            }}
          >
            {copied ? <Check size={14} color="#3fb950" /> : <Copy size={14} />}
            <span>{copied ? 'Copied' : 'Copy'}</span>
          </button>

          <button
            onClick={resetCode}
            title="Reset to Template"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              backgroundColor: 'rgba(255, 255, 255, 0.05)',
              color: 'rgba(255, 255, 255, 0.8)',
              border: '0.5px solid rgba(255, 255, 255, 0.15)',
              borderRadius: '6px',
              padding: '6px 10px',
              fontSize: '12px',
              cursor: 'pointer',
              transition: 'all 0.2s',
            }}
          >
            <RotateCcw size={14} />
            <span>Reset</span>
          </button>

          {/* Run Button */}
          <button
            onClick={runCode}
            disabled={!wasmReady || isRunning}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              background: wasmReady ? '#FF0068' : '#30363d',
              color: '#ffffff',
              border: 'none',
              borderRadius: '6px',
              padding: '6px 16px',
              fontSize: '12px',
              fontWeight: 600,
              cursor: wasmReady && !isRunning ? 'pointer' : 'not-allowed',
              boxShadow: wasmReady ? '0 2px 10px rgba(255, 0, 104, 0.35)' : 'none',
              transition: 'transform 0.15s, background-color 0.15s',
            }}
            onMouseEnter={(e) => {
              if (wasmReady && !isRunning) e.currentTarget.style.backgroundColor = '#e6005c';
            }}
            onMouseLeave={(e) => {
              if (wasmReady && !isRunning) e.currentTarget.style.backgroundColor = '#FF0068';
            }}
          >
            <Play size={13} fill="currentColor" />
            <span>{isRunning ? 'Running...' : 'Run Code (Ctrl+↵)'}</span>
          </button>
        </div>
      </div>

      {/* Main Workspace (Editor + Output Terminal) */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'minmax(0, 1.2fr) minmax(0, 0.8fr)',
        flex: 1,
        minHeight: '480px',
        backgroundColor: '#000000',
      }}>
        {/* Monaco Editor Pane */}
        <div style={{
          borderRight: '0.5px solid rgba(255, 255, 255, 0.1)',
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: '#000000',
          position: 'relative',
        }}>
          <Editor
            height="100%"
            defaultLanguage="flame"
            theme="github-dark"
            value={code}
            onChange={(val) => setCode(val || '')}
            beforeMount={handleEditorWillMount}
            onMount={(editor, monaco) => {
              editorRef.current = editor;
              editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
                runCode();
              });

              // Remeasure fonts once DOM and web fonts are fully settled
              if (typeof document !== 'undefined' && document.fonts) {
                document.fonts.ready.then(() => {
                  monaco.editor.remeasureFonts();
                });
              }

              // Double-check remeasurement and layout to eliminate any vertical click-to-line offset
              setTimeout(() => {
                monaco.editor.remeasureFonts();
                editor.layout();
              }, 120);
            }}
            options={{
              fontSize: 14,
              lineHeight: 22,
              fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace",
              fontLigatures: true,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              automaticLayout: true,
              fixedOverflowWidgets: true,
              tabSize: 4,
              suggestOnTriggerCharacters: true,
              quickSuggestions: true,
              suggest: {
                preview: true,
                showIcons: true,
                showWords: true,
                filterGraceful: true,
                snippetsPreventQuickSuggestions: false,
              },
              padding: { top: 12, bottom: 12 },
              cursorBlinking: 'smooth',
              renderLineHighlight: 'all',
              lineNumbersMinChars: 3,
            }}
          />
        </div>

        {/* Output Console Pane */}
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: '#000000',
          color: '#e6edf3',
          fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, monospace",
        }}>
          {/* Console Header */}
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '10px 16px',
            backgroundColor: '#000000',
            borderBottom: '0.5px solid rgba(255, 255, 255, 0.1)',
            fontSize: '12px',
            color: 'rgba(255, 255, 255, 0.7)',
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Terminal size={14} color="#FF0068" />
              <span style={{ fontWeight: 600, color: '#ffffff' }}>Terminal Output</span>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              {executionTime !== null && (
                <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  color: 'rgba(255, 255, 255, 0.8)',
                  backgroundColor: 'rgba(255, 255, 255, 0.05)',
                  padding: '2px 8px',
                  borderRadius: '4px',
                  border: '0.5px solid rgba(255, 255, 255, 0.1)',
                }}>
                  <Clock size={12} color="#79c0ff" />
                  <span>{executionTime} ms</span>
                </div>
              )}

              {isSuccess !== null && (
                <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  color: isSuccess ? '#3fb950' : '#f85149',
                  backgroundColor: isSuccess ? 'rgba(63, 185, 80, 0.1)' : 'rgba(248, 81, 73, 0.1)',
                  padding: '2px 8px',
                  borderRadius: '4px',
                  border: `0.5px solid ${isSuccess ? 'rgba(63, 185, 80, 0.3)' : 'rgba(248, 81, 73, 0.3)'}`,
                  fontWeight: 500,
                }}>
                  {isSuccess ? <CheckCircle2 size={13} /> : <AlertCircle size={13} />}
                  <span>{isSuccess ? 'Success' : 'Error'}</span>
                </div>
              )}

              <button
                onClick={() => setOutput('')}
                style={{
                  background: 'none',
                  border: 'none',
                  color: 'rgba(255, 255, 255, 0.5)',
                  fontSize: '11px',
                  cursor: 'pointer',
                  padding: '2px 6px',
                  borderRadius: '4px',
                  transition: 'color 0.15s',
                }}
                onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'}
                onMouseLeave={(e) => e.currentTarget.style.color = 'rgba(255, 255, 255, 0.5)'}
              >
                Clear
              </button>
            </div>
          </div>

          {/* Console Log Area */}
          <div style={{
            flex: 1,
            padding: '16px',
            overflowY: 'auto',
            fontSize: '13px',
            lineHeight: '1.6',
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-all',
            backgroundColor: '#000000',
            color: isSuccess === false ? '#ff7b72' : '#e6edf3',
          }}>
            {output}
          </div>

          {/* Console Footer / Host info */}
          <div style={{
            padding: '8px 16px',
            backgroundColor: '#000000',
            borderTop: '0.5px solid rgba(255, 255, 255, 0.1)',
            fontSize: '11px',
            color: 'rgba(255, 255, 255, 0.4)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}>
            <span>Blaze VM v0.6.3 (WebAssembly Target)</span>
            <span>Sandboxed In-Browser Execution</span>
          </div>
        </div>
      </div>
    </div>
  );
}
