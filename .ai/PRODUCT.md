# Product Overview

## Summary

`test-popup-ai` e un assistente AI desktop leggero che compare con una scorciatoia globale e permette di interrogare un backend AI senza interrompere il flusso di lavoro.

## Product Goal

Ridurre al minimo il contesto operativo necessario per usare un modello AI durante attivita quotidiane come scrittura, riscrittura, traduzione, debugging e quick assistance.
Il focus primario del prodotto e aiutare a ripulire, ottimizzare, tradurre e adattare testo gia scritto o da rifinire, in modo che sia subito riutilizzabile.

## Core Value Proposition

- Accesso immediato all'AI senza browser o cambio finestra
- Supporto a backend cloud e locali
- Configurazione semplice via file YAML
- Integrazione pratica con il flusso desktop

## Primary Use Cases

- Riformulare testo selezionato
- Tradurre rapidamente contenuti
- Generare risposta da reinserire nell'applicazione attiva
- Usare alias di prompt per task ripetitivi
- Consultare un backend locale senza uscire dall'ambiente di lavoro

## Non-Goals

- Non sostituisce un client chat completo con cronologia avanzata
- Non e un editor ricco o un IDE assistant full-context
- Non punta a un sistema plugin complesso in questa fase iniziale

## UX Principles

- Apertura istantanea
- Interazione essenziale
- Chiarezza nello stato di caricamento
- Feedback esplicito quando l'auto-apply non e disponibile
- Identita visiva configurabile ma coerente

## Constraints

- Wayland limita alcune forme di automazione/input injection
- L'auto-apply dipende da utility di sistema esterne
- Il popup deve restare leggero, prevedibile e trasportabile
