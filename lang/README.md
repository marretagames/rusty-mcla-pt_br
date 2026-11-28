# Tradução do jogo em pt-BR

O jogo lê os textos das tabelas de strings do **disco** (inglês, espanhol, francês, alemão e italiano). O
português do Brasil não existe no disco, então ele funciona como uma **camada sobre o inglês**: o jogo roda em
inglês por baixo (filmes de interface, save, fontes) e os textos que você traduziu substituem os originais.

## Arquivos (os dois vão no repositório e no release)

- `lang/pt-br.tsv`: `chave<TAB>texto em português` (menus, HUD, mensagens). `\n` = quebra de linha. As linhas que começam com `@` (`@inglês<TAB>português`) são textos que o próprio port escreve no código (como DISPLAY e PERFORMANCE); `{}` dentro delas é um número ou nome que o port preenche.
- `lang/pt-br-subtitles.tsv`: `CENA<TAB>tempo inicial<TAB>texto` (legendas das cutscenes da história).

O que não estiver nesses arquivos continua em inglês. O release copia os dois para a pasta do jogo, ao lado do
`mcla.exe`. Eles **só têm chave + português**: o texto original do jogo não deve ser publicado.

## Como traduzir

1. Compile a ferramenta: `cd mcla-rs && cargo build --release -p mcla-tool`
2. Extraia os textos em inglês do **seu** disco já instalado (pasta `data` do jogo):
   `python tools/ptbr.py extract --data "C:\caminho\para\data"`
   Isso cria `work/pt-br-work.tsv` e `work/pt-br-subtitles-work.tsv` (a pasta `work/` está no `.gitignore`).
3. Abra os arquivos de `work/` numa planilha (separador TAB, UTF-8) e preencha a **última coluna** em português.
4. Gere os arquivos do repositório: `python tools/ptbr.py export`
5. Faça commit de `lang/` e publique o release. No launcher, em Configurações, escolha **Português (BR)**.

Ao rodar de novo o `extract`, o que você já traduziu em `lang/` volta para as planilhas.

## Verificar no jogo

O log do jogo mostra `pt-BR text: N entries from ...` quando carregou. Se aparecer `not found`, o arquivo não está
ao lado do `mcla.exe`. Confira se **ã, õ, ç, é, á** aparecem corretamente nos menus, no HUD e nas legendas; se alguma
fonte do jogo não tiver o caractere, ele sai errado ou em branco.
