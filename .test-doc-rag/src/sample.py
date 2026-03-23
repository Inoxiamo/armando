def rag_runtime_override_examples():
    """Example snippet for retrieval."""
    cases = [
        "!rag on explain how the vector DB works",
        "!rag off answer without retrieval",
    ]
    return cases


if __name__ == "__main__":
    for item in rag_runtime_override_examples():
        print(item)
