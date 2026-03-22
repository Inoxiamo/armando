public class SampleService {
    public String explain() {
        return "Retrieval uses query/document embeddings and ordering by cosine score.";
    }

    public String rankingNote() {
        return "Score close to 1.0 => chunk very similar to the query.";
    }
}
