import random
import nltk
from nltk.corpus import brown

# Baixar os dados necessários
nltk.download('brown')
nltk.download('universal_tagset')
nltk.download('words')

# Carregar as 10.000 palavras mais comuns em inglês
common_words = nltk.FreqDist(w.lower() for w in brown.words()).most_common(10000)
common_words = [word for word, _ in common_words]

# Função para obter uma lista de palavras de um tipo específico
def get_words_by_tag(tag):
    words = set()
    for word, pos in brown.tagged_words(tagset='universal'):
        if pos == tag and word.lower() in common_words:
            words.add(word.lower())
    return list(words)

# Obter listas de substantivos, verbos e adjetivos
subjects = get_words_by_tag('NOUN')  # Substantivos
verbs = get_words_by_tag('VERB')    # Verbos
objects = get_words_by_tag('ADJ')   # Adjetivos

# Limitar as listas para garantir variedade
subjects = random.sample(subjects, 1000) if len(subjects) > 1000 else subjects
verbs = random.sample(verbs, 1000) if len(verbs) > 1000 else verbs
objects = random.sample(objects, 1000) if len(objects) > 1000 else objects

# Função para gerar frases com no máximo 24 palavras
def generate_sentences(subjects, verbs, objects, n):
    sentences = set()
    while len(sentences) < n:
        subject = random.choice(subjects)
        verb = random.choice(verbs)
        object_ = random.choice(objects)
        sentence_length = random.randint(3, 24)  # Comprimento da frase entre 5 e 24 palavras
        sentence = f"{subject} {verb} {object_}"
        for _ in range(sentence_length - 3):
            word_type = random.choice([subjects, verbs, objects])
            sentence += f" {random.choice(word_type)}"
        sentences.add(sentence)
    return list(sentences)

# Geração de 1000 frases
sentences = generate_sentences(subjects, verbs, objects, 100000)

# Salvar as frases em um arquivo
file_path = "sentences_nltk_common_1000_v2.txt"
with open(file_path, "w") as f:
    for sentence in sentences:
        f.write(sentence + "\n")