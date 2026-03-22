def rag_runtime_override_examples():
    """Snippet di esempio per retrieval."""
    cases = [
        "!rag on spiegami come funziona il vector db",
        "!rag off rispondi senza retrieval",
    ]
    return cases


if __name__ == "__main__":
    for item in rag_runtime_override_examples():
        print(item)
