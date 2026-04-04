import devSamTheme from "dev-sam-theme/themes/dev-sam-theme.json";
import * as shiki from "shiki";
import samlangGrammar from "./samlang-syntax.json";

type Props = {
  readonly language: string;
  readonly className?: string;
  readonly transparent?: boolean;
  readonly manualSection?: React.ReactNode;
  readonly children: string;
};

const DARK_COLOR_MAP: Record<string, string> = {
  "#333333": "#c8d2d7",
  "#38484f": "#c8d2d7",
  "#3e7ae2": "#6eaaff",
  "#9a30ad": "#c878dc",
  "#c33b30": "#f07864",
  "#1a8f52": "#50c882",
  "#50a14f": "#50c882",
  "#d52262": "#f06496",
  "#808080": "#969696",
  "#a0a1a7": "#b0b1b7",
  "#0184bc": "#56c8f0",
};

function darkColorFor(lightColor: string | undefined): string | undefined {
  if (!lightColor) return undefined;
  return DARK_COLOR_MAP[lightColor.toLowerCase()] ?? lightColor;
}

function Token({ token }: { token: shiki.ThemedToken }): React.JSX.Element {
  const darkColor = darkColorFor(token.color);
  const style: React.CSSProperties & Record<string, string> = {
    "--token-color": token.color ?? "",
    "--token-color-dark": darkColor ?? "",
  };
  if (token.fontStyle === 2) style.fontWeight = "bold";
  if (token.fontStyle === 1) style.fontStyle = "italic";
  return (
    <span className="code-token" style={style}>
      {token.content}
    </span>
  );
}

const lang: shiki.LanguageRegistration = {
  // biome-ignore lint/suspicious/noExplicitAny: <explanation>
  ...(samlangGrammar as any),
  name: "samlang",
  scopeName: "text.samlang",
};

export default async function StaticCodeBlock({
  language,
  className,
  transparent = false,
  manualSection,
  children,
}: Props): Promise<React.JSX.Element> {
  const theme: shiki.ThemeInput = {
    ...devSamTheme,
    settings: devSamTheme.tokenColors,
    name: "dev-sam-theme",
    bg: transparent ? "transparent" : "#F7F7F7",
    type: "light",
  };
  const highlighter = await shiki.getSingletonHighlighter({
    themes: [theme],
    langs: [lang, "bash", "js", "json", "rust", "typescript", "tsx"],
  });

  const tokens = highlighter.codeToTokens(children, {
    lang: language as shiki.BuiltinLanguage,
    theme: "dev-sam-theme",
  });

  const lightBg = transparent ? "transparent" : "#F7F7F7";
  const darkBg = transparent ? "transparent" : "#2a2a2a";
  const preStyle: React.CSSProperties & Record<string, string> = {
    "--pre-fg": "#38484f",
    "--pre-fg-dark": "#c8d2d7",
    "--pre-bg": lightBg,
    "--pre-bg-dark": darkBg,
  };

  return (
    <pre className={`code-block ${className ?? ""}`.trim()} style={preStyle}>
      {manualSection}
      {tokens.tokens.map((line, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: only valid key
        <span key={i}>
          {line.map((token, j) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: only valid key
            <Token key={j} token={token} />
          ))}
          {"\n"}
        </span>
      ))}
    </pre>
  );
}
