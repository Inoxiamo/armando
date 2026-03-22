public class SampleService {
    public String explain() {
        return "Il retrieval usa embedding query/documento e ordinamento per score coseno.";
    }

    public String rankingNote() {
        return "Score vicino a 1.0 => chunk molto simile alla query.";
    }
}
