// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import flameGrammar from './src/syntax/flame.tmLanguage.json' with { type: 'json' };
import fmiGrammar from './src/syntax/fmi.tmLanguage.json' with { type: 'json' };
import githubDark from '@shikijs/themes/github-dark';
import githubLight from '@shikijs/themes/github-light';
import { ExpressiveCodeTheme } from '@expressive-code/core';

import react from '@astrojs/react';

const operatorScopes = [
    'keyword.operator',
    'keyword.operator.flame',
    'keyword.operator.arithmetic.flame',
    'keyword.operator.comparison.flame',
    'keyword.operator.assignment.flame',
    'keyword.operator.assignment.compound.flame',
    'keyword.operator.logical.flame',
    'keyword.operator.bitwise.flame',
    'keyword.operator.bitwise.shift.flame',
    'keyword.operator.increment.flame',
    'keyword.operator.decrement.flame',
    'keyword.operator.arrow.flame',
    'keyword.operator.arrow.fat.flame',
    'keyword.operator.pipe.flame',
    'keyword.operator.range.flame',
    'keyword.operator.nil.flame'
];

const flameDarkTheme = new ExpressiveCodeTheme({
    ...githubDark,
    name: 'github-dark',
    tokenColors: [
        ...(githubDark.tokenColors || []),
        {
            scope: operatorScopes,
            settings: {
                foreground: '#FF453A'
            }
        }
    ]
});

const flameLightTheme = new ExpressiveCodeTheme({
    ...githubLight,
    name: 'github-light',
    tokenColors: [
        ...(githubLight.tokenColors || []),
        {
            scope: operatorScopes,
            settings: {
                foreground: '#D73A49'
            }
        }
    ]
});

// https://astro.build/config
export default defineConfig({
    vite: {
        server: {
            fs: {
                allow: ['..']
            }
        }
    },
    markdown: {
        shikiConfig: {
            langs: [
                // @ts-ignore
                {
                    ...flameGrammar,
                    name: 'flame',
                    aliases: ['fm', 'flamelang'],
                },
                // @ts-ignore
                {
                    ...fmiGrammar,
                    name: 'fmi',
                    aliases: ['flame-interface'],
                },
            ],
        },
    },
    integrations: [starlight({
        title: 'Flame',
        components: {
            Hero: './src/components/CustomHero.astro',
        },
        expressiveCode: {
            themes: [flameDarkTheme, flameLightTheme],
        },
        logo: {
            src: './src/assets/flame.png',
            alt: 'Flame Logo',
        },
        head: [
            {
                tag: 'meta',
                attrs: {
                    name: 'google-site-verification',
                    content: 'N9sdgiZupGzt7Ib-WtTXDBF78oGUvM5EEJ4UcQWCSTQ',
                },
            },
        ],
        customCss: ['./src/styles/custom.css'],
        social: [
            { icon: 'github', label: 'GitHub', href: 'https://github.com/sohamglx/flame' }
        ],
        sidebar: [
            {
                label: 'Getting Started',
                items: [
                    { label: 'Introduction', slug: 'getting-started/introduction' },
                    { label: 'Installation & Toolchain', slug: 'getting-started/installation' },
                    { label: 'Architecture & Analysis', slug: 'getting-started/architecture' },
                    { label: 'Production & Docker', slug: 'getting-started/deployment-and-docker' },
                    { label: 'Tutorial: Telegram Bot', slug: 'getting-started/telegram-bot' },
                    { label: 'Interactive Playground', slug: 'playground' },
                ],
            },
            {
                label: 'Language Basics',
                collapsed: true,
                items: [
                    { label: 'Variables & Constants', slug: 'language-basics/variables' },
                    { label: 'Data Types & Formulas', slug: 'language-basics/data-types' },
                    { label: 'Operators & Expressions', slug: 'language-basics/operators' },
                    { label: 'Nil Safety & Optionals', slug: 'language-basics/nil-safety' },
                    { label: 'Type Conversions', slug: 'language-basics/type-conversions' },
                    { label: 'Control Flow', slug: 'language-basics/control-flow' },
                    { label: 'Functions & Closures', slug: 'language-basics/functions' },
                    { label: 'Application Entry Point', slug: 'language-basics/application' },
                ],
            },
            {
                label: 'Types & Object-Oriented',
                collapsed: true,
                items: [
                    { label: 'Structs', slug: 'types-and-traits/structs' },
                    { label: 'Impl & Methods', slug: 'types-and-traits/impl-and-methods' },
                    { label: 'Enums & Patterns', slug: 'types-and-traits/enums' },
                    { label: 'Traits & Interfaces', slug: 'types-and-traits/traits' },
                    { label: 'Generics & Type Parameters', slug: 'types-and-traits/generics' },
                ],
            },
            {
                label: 'Memory & Safety',
                collapsed: true,
                items: [
                    { label: 'Ownership & Move Semantics', slug: 'memory-and-safety/ownership' },
                    { label: 'Borrowing & References', slug: 'memory-and-safety/borrowing' },
                ],
            },
            {
                label: 'Concurrency',
                collapsed: true,
                items: [
                    { label: 'Async & Await (I/O)', slug: 'concurrency/async-await' },
                ],
            },
            {
                label: 'Packages & Native Rust',
                collapsed: true,
                items: [
                    { label: 'flame.toml Reference', slug: 'packages-and-native/flame-toml' },
                    { label: 'Modules & Imports', slug: 'packages-and-native/modules-and-imports' },
                    { label: 'Creating Packages', slug: 'packages-and-native/creating-packages' },
                    { label: 'Using Native Rust Crates', slug: 'packages-and-native/native-rust-crates' },
                    { label: 'Native Plugins', slug: 'packages-and-native/native-plugins' },
                    { label: 'Native Macros (flame-macro)', slug: 'packages-and-native/native-macros' },
                    { label: 'Rust Integration (flamebinder)', slug: 'packages-and-native/rust-integration' },
                ],
            },
            {
                label: 'Annotations & Testing',
                collapsed: true,
                items: [
                    { label: 'Built-in Annotations & CLI', slug: 'annotations-and-testing/builtin-annotations' },
                    { label: 'Custom Annotations & Scope Injection', slug: 'annotations-and-testing/custom-annotations' },
                    { label: 'Testing Framework', slug: 'annotations-and-testing/testing-framework' },
                ],
            },
            {
                label: 'Standard Library',
                collapsed: true,
                items: [
                    { label: 'Overview', slug: 'std/overview' },
                    { label: "Formatting (std.fmt)", slug: 'std/fmt' },
                    { label: 'JSON (std.json)', slug: 'std/json' },
                    { label: 'Environment Variables (std.env)', slug: 'std/env' },
                    { label: 'File System (std.fs)', slug: 'std/filesystem' },
                    { label: 'Networking (std.net)', slug: 'std/net' },
                    { label: 'WebSockets (std.net.ws)', slug: 'std/websocket' },
                    { label: 'Process (std.process)', slug: 'std/process' },
                    { label: 'OS Introspection (std.os)', slug: 'std/os' },
                    { label: 'Byte Manipulation (std.byte)', slug: 'std/byte' },
                    { label: 'Desktop Automation (std.desktop)', slug: 'std/desktop' },
                    { label: 'Window Management (std.window)', slug: 'std/window' },
                    { label: 'Threading (std.thread)', slug: 'std/thread' },
                    { label: 'Time (std.time)', slug: 'std/time' },
                    { label: 'Math (std.math)', slug: 'std/math' },
                    { label: 'Unit System (std.unit)', slug: 'std/unit' },
                    { label: 'Annotations & Metaprogramming (std.annotation)', slug: 'std/annotation' },
                    { label: 'Camera (std.camera)', slug: 'std/camera' },
                    { label: 'Web Applications (std.web)', slug: 'std/web' },
                ],
            },
        ],
    }), react()],
});
