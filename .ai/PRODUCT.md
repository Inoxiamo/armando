# Product Overview

## Summary

`test-popup-ai` is a lightweight desktop AI assistant that opens from the operating system and lets the user query an AI backend without breaking their working flow.
It is optimized for quick text work, low-friction settings changes, and desktop-native behavior instead of browser-style session management.

## Product Goal

Minimize the operational overhead required to use an AI model during everyday tasks such as writing, rewriting, translation, debugging, and quick assistance.
The main product focus is helping users clean up, optimize, translate, and adapt text so the result can be reused immediately.

## Core Value Proposition

- Immediate AI access without opening a browser or changing windows
- Support for both cloud and local backends
- Simple configuration through YAML files and the built-in settings panel
- Practical integration with desktop-first workflows
- Visual consistency that keeps the popup readable and dependable during repeated daily use

## Primary Use Cases

- Rewrite selected text
- Translate content quickly
- Generate a response to paste back into the active application
- Use prompt aliases for repetitive tasks
- Query a local backend without leaving the working environment

## Non-Goals

- It is not a full chat client with advanced conversation management
- It is not a rich editor or a full-context IDE assistant
- It does not aim for a complex plugin system at this early stage
- It does not execute external commands or tools automatically without explicit confirmation UX

## UX Principles

- Instant launch
- Minimal interaction overhead
- Clear loading and processing states
- Clear separation between configuration, generated content, and history
- Configurable but cohesive visual identity
- Desktop integration should feel native enough to launch from shortcuts, menus, and taskbars without extra manual setup

## Constraints

- The popup must remain lightweight, predictable, and portable
- Future tool-based integrations require sandboxing and explicit user consent
