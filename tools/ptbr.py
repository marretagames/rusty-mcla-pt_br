#!/usr/bin/env python3
"""Ferramenta de tradução pt-BR do Rusty MCLA.

  python tools/ptbr.py extract --data <pasta data do jogo>
      Lê os textos em inglês do SEU disco (via mcla-tool) e cria os arquivos de trabalho em work/
      (não publique a pasta work/: ela tem o texto original do jogo).

  python tools/ptbr.py export
      Lê work/*.tsv e grava lang/pt-br.tsv e lang/pt-br-subtitles.tsv só com a sua tradução
      (chave + português), que são os arquivos que vão para o repositório e para o release.
"""
import argparse, csv, io, os, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "work"
LANG = ROOT / "lang"
SKIP_ROWS = {"position", "size", "", "time correction"}


def fields(line, sep):
    """Mesma regra do jogo (cutscene.rs `fields`): aspas alternam o modo e são descartadas."""
    out, cur, quoted = [], [], False
    for c in line:
        if c == '"':
            quoted = not quoted
        elif c == sep and not quoted:
            out.append("".join(cur)); cur = []
        else:
            cur.append(c)
    out.append("".join(cur))
    return out


def read_tsv(path):
    rows = {}
    if path.exists():
        for line in path.read_text(encoding="utf-8-sig").splitlines():
            if line and not line.startswith("#") and "\t" in line:
                rows[tuple(line.split("\t")[:-1])] = line.split("\t")[-1]
    return rows


def find_tool(arg):
    if arg:
        return arg
    for n in ("mcla-tool.exe", "mcla-tool"):
        p = ROOT / "mcla-rs" / "target" / "release" / n
        if p.exists():
            return str(p)
    sys.exit("mcla-tool não encontrado. Compile com: cd mcla-rs && cargo build --release -p mcla-tool (ou use --tool <caminho>)")


def subtitle_rows(scene_dir):
    """(início, inglês) de cada legenda de uma cena, como o jogo as lê (coluna do inglês)."""
    txt, csv_ = scene_dir / "subtitle.txt", scene_dir / "subtitle.csv"
    if txt.exists():
        b = txt.read_bytes()
        text, sep = b.decode("utf-16-le", errors="replace").lstrip("\ufeff"), "\t"
    elif csv_.exists():
        text, sep = csv_.read_bytes().decode("latin-1"), ","
    else:
        return []
    out = []
    for line in text.splitlines()[1:]:
        f = fields(line.rstrip("\r"), sep)
        if len(f) < 2:
            continue
        try:
            float(f[0].strip())
        except ValueError:
            continue
        if f[1].strip() in SKIP_ROWS:
            continue
        en = f[1].strip()
        if len(f) > 1 and en:
            out.append((f[0].strip(), en.replace("\n", "\\n")))
    return out


def extract(a):
    data = Path(a.data)
    WORK.mkdir(exist_ok=True)
    tool = find_tool(a.tool)
    raw = WORK / "strings-en.tsv"
    subprocess.run([tool, "strings", str(data / "ui"), str(raw)], check=True)
    old = {k[0]: v for k, v in read_tsv(LANG / "pt-br.tsv").items()}
    seen, lines = set(), ["# chave<TAB>inglês (só consulta)<TAB>português (preencha)"]
    for line in raw.read_text(encoding="utf-8").splitlines():
        p = line.split("\t", 2)
        if len(p) != 3 or p[1] in seen:
            continue
        seen.add(p[1])
        lines.append(f"{p[1]}\t{p[2]}\t{old.get(p[1], '')}")
    (WORK / "pt-br-work.tsv").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{len(lines) - 1} textos em work/pt-br-work.tsv")

    old_s = read_tsv(LANG / "pt-br-subtitles.tsv")
    slines, n = ["# cena<TAB>início<TAB>inglês (só consulta)<TAB>português (preencha)"], 0
    base = data / "tune" / "cutscene" / "Story"
    for d in sorted(p for p in base.iterdir() if p.is_dir() and p.name.upper().startswith("CUT_")) if base.exists() else []:
        for start, en in subtitle_rows(d):
            slines.append(f"{d.name.upper()}\t{start}\t{en}\t{old_s.get((d.name.upper(), start), '')}")
            n += 1
    (WORK / "pt-br-subtitles-work.tsv").write_text("\n".join(slines) + "\n", encoding="utf-8")
    print(f"{n} legendas em work/pt-br-subtitles-work.tsv")


def export(_a):
    LANG.mkdir(exist_ok=True)
    w = WORK / "pt-br-work.tsv"
    if w.exists():
        out = ["# Tradução pt-BR dos textos do jogo. Formato: chave<TAB>português. \\n = quebra de linha."]
        for line in w.read_text(encoding="utf-8").splitlines():
            p = line.split("\t")
            if len(p) >= 3 and not line.startswith("#") and p[2].strip():
                out.append(f"{p[0]}\t{p[2].strip()}")
        # (as linhas "@inglês<TAB>português" são os textos do próprio port e ficam como estão)
        keep = [l for l in (LANG / "pt-br.tsv").read_text(encoding="utf-8").splitlines() if l.startswith("@")] if (LANG / "pt-br.tsv").exists() else []
        (LANG / "pt-br.tsv").write_text("\n".join(out + keep) + "\n", encoding="utf-8")
        print(f"{len(out) - 1} textos -> lang/pt-br.tsv")
    s = WORK / "pt-br-subtitles-work.tsv"
    if s.exists():
        out = ["# Legendas pt-BR. Formato: CENA<TAB>início<TAB>português. \\n = quebra de linha."]
        for line in s.read_text(encoding="utf-8").splitlines():
            p = line.split("\t")
            if len(p) >= 4 and not line.startswith("#") and p[3].strip():
                out.append(f"{p[0]}\t{p[1]}\t{p[3].strip()}")
        (LANG / "pt-br-subtitles.tsv").write_text("\n".join(out) + "\n", encoding="utf-8")
        print(f"{len(out) - 1} legendas -> lang/pt-br-subtitles.tsv")


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest="cmd", required=True)
    e = sp.add_parser("extract"); e.add_argument("--data", required=True); e.add_argument("--tool"); e.set_defaults(f=extract)
    x = sp.add_parser("export"); x.set_defaults(f=export)
    a = ap.parse_args(); a.f(a)
