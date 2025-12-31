import type { Metadata } from "next";
import Link from "next/link";
import Script from "next/script";
import type { ReactNode } from "react";
import NavBar from "../lib/NavBar";
import StaticCodeBlock from "../lib/StaticCodeBlock";
import { DATASET_TIMELINE } from "../lib/home-timeline-data";

function LazyCardMedia({ image, title }: { image: string; title: string }): React.JSX.Element {
  return <img src={image} alt={title} title={title} loading="lazy" />;
}

function ButtonLink({
  href,
  children,
  className,
}: {
  href: string;
  className?: string;
  children: ReactNode;
}): React.JSX.Element {
  const classes = [
    "button bg-transparent border-0 cursor-pointer px-6 py-1.5 font-bold text-sm text-center rounded",
    "hover:bg-blue-500 hover:bg-opacity-10 transition-colors duration-200",
  ];
  if (className != null) {
    classes.push(className);
  }
  return (
    <Link className={classes.join(" ")} href={href}>
      {typeof children === "string" ? children.toLocaleUpperCase() : children}
    </Link>
  );
}

function Card({
  className,
  children,
}: { className?: string; children: ReactNode }): React.JSX.Element {
  const CardBaseCSS =
    "flex flex-col bg-white rounded filter drop-shadow hover:drop-shadow-lg transition-all duration-300 ease-out";
  const classes = className != null ? `${CardBaseCSS} ${className}` : CardBaseCSS;
  return <div className={classes}>{children}</div>;
}

function CardHeader({
  title,
  subheader,
}: { title: string; subheader?: string }): React.JSX.Element {
  return (
    <div className="px-4 pb-0 last:pb-4">
      <h4 className="mb-0">{title}</h4>
      {subheader && <small>{subheader}</small>}
    </div>
  );
}

function CardContainer({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}): React.JSX.Element {
  const CardContainerBaseCSS = "flex flex-row flex-wrap justify-center max-w-7xl mx-auto";
  const classes = className != null ? `${CardContainerBaseCSS} ${className}` : CardContainerBaseCSS;
  return <div className={classes}>{children}</div>;
}

function StickyCodeBlock(): React.JSX.Element {
  const code = `
import {List} from std.list;

class Developer(
  val github: Str,
  val projects: List<Str>,
) {
  function sam(): Developer = {
    let github = "SamChou19815";
    let projects = List
      .of("samlang")
      .cons("website")
      .cons("...");
    Developer.init(github, projects)
  }
}

class Main {
  function main(): Developer = Developer.sam()
}`;
  return (
    <>
      <StaticCodeBlock
        language="samlang"
        transparent
        className="mx-auto my-0 text-xs leading-5 sm:text-sm"
        manualSection={
          <>
            <div className="token-line">
              <span className="token comment">/**</span>
            </div>
            <div className="token-line">
              <span className="token comment">
                {" "}
                * Copyright (C) 2015-{new Date().getFullYear()} Developer Sam.
              </span>
            </div>
            <div className="token-line">
              <span className="token comment">
                {" * @demo "}
                <a
                  className="text-[var(--prism-code-block-comment-color)] underline"
                  href="https://samlang.io/demo"
                >
                  https://samlang.io/demo
                </a>
              </span>
            </div>
            <div className="token-line">
              <span className="token comment">
                {" * @github "}
                <a
                  className="text-[var(--prism-code-block-comment-color)] underline"
                  href="https://github.com/SamChou19815"
                >
                  https://github.com/SamChou19815
                </a>
              </span>
            </div>
            <div className="token-line">
              <span className="token comment">
                {" * @bsky "}
                <a
                  className="text-[var(--prism-code-block-comment-color)] underline"
                  href="https://bsky.app/profile/developersam.com"
                >
                  https://bsky.app/profile/developersam.com
                </a>
              </span>
            </div>
            <div className="token-line">
              <span className="token comment">
                {" "}
                * @resume{" "}
                <a
                  className="text-[var(--prism-code-block-comment-color)] underline"
                  href="https://developersam.com/resume.pdf"
                >
                  https://developersam.com/resume.pdf
                </a>
              </span>
            </div>
            <div className="token-line">
              <span className="token comment"> */</span>
            </div>
          </>
        }
      >
        {code}
      </StaticCodeBlock>
    </>
  );
}

const TimelineSection = (
  <section>
    <div className="relative flex flex-row flex-wrap items-center justify-center">
      <div className="lg:left-[calc(50%-255px)] absolute top-8 bottom-36 hidden w-0.5 bg-blue-500 lg:block" />
      {DATASET_TIMELINE.map((item) => (
        <CardContainer key={`${item.title}-${item.time}`} className="mb-4 w-full">
          <div className="mx-auto my-0 flex w-full content-start items-start justify-center">
            <span className="connector-dot mt-6 ml-2 mr-4 hidden h-2 w-2 rounded bg-blue-500 lg:block" />
            <Card className="lg:w-[500px] ml-0 w-11/12">
              {item.image != null && <LazyCardMedia image={item.image} title={item.title} />}
              <CardHeader title={item.title} subheader={item.time} />
              {item.detail != null && <div className="p-4 pb-0 last:pb-4">{item.detail}</div>}
              {item.links != null && (
                <div className="p-4">
                  {item.links.map(({ name, url }, index) => (
                    // biome-ignore lint/suspicious/noArrayIndexKey: no other good choice
                    <ButtonLink key={index} href={url} className="px-1.5">
                      {name}
                    </ButtonLink>
                  ))}
                </div>
              )}
            </Card>
          </div>
        </CardContainer>
      ))}
    </div>
  </section>
);

export const metadata: Metadata = {
  title: "Developer Sam",
  description: "Explore the portfolio and projects created and open sourced by Developer Sam.",
  authors: { name: "Developer Sam" },
  openGraph: {
    images: "https://developersam.com/sam-by-megan-3-square.webp",
    type: "profile",
  },
};

export default function IndexPage(): React.JSX.Element {
  return (
    <>
      <Script
        type="application/ld+json"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: static scripts
        dangerouslySetInnerHTML={{
          __html: `{"@context":"http://schema.org","@type":"Organization","url":"https://developersam.com","logo":"https://developersam.com/logo.png"}`,
        }}
      />
      <Script
        type="application/ld+json"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: static scripts
        dangerouslySetInnerHTML={{
          __html: `{"@context":"http://schema.org","@type":"Person","name":"Developer Sam","url":"https://developersam.com","sameAs":[
  "https://www.developersam.com",
  "https://blog.developersam.com",
  "https://www.facebook.com/SamChou19815",
  "https://twitter.com/SamChou19815",
  "https://github.com/SamChou19815",
  "https://www.linkedin.com/in/sam-zhou-30b91610b/"]}`,
        }}
      />
      <NavBar title="Developer Sam" titleLink="/" navItems={[{ name: "Blog", link: "/blog" }]} />
      <div className="relative mx-auto flex max-w-7xl flex-row flex-wrap justify-start lg:flex-nowrap">
        <div className="lg:w-[550px] flex h-[calc(100vh-8rem)] w-full flex-wrap items-center lg:sticky lg:top-0">
          <StickyCodeBlock />
        </div>
        <div className="my-12">{TimelineSection}</div>
      </div>
    </>
  );
}
