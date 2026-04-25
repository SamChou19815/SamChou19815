import type { MDXComponents } from "mdx/types";
import Link from "next/link";
import { isValidElement } from "react";
import StaticCodeBlock from "./lib/StaticCodeBlock";

const CustomMDXComponents: MDXComponents = {
  a: ({ href, ...props }) => {
    if (href?.startsWith("http")) {
      return <a href={href} {...props} />;
    }
    return <Link href={href || ""} />;
  },
  pre: (props) => {
    // oxlint-disable-next-line no-explicit-any
    const stringOrComponentChildren = props.children as any;
    const { className, children } = (
      isValidElement(stringOrComponentChildren) ? stringOrComponentChildren.props : props
    ) as { className: string; readonly children: string };
    return (
      <StaticCodeBlock language={className.replace(/language-/, "")} className="text-sm">
        {children.trim()}
      </StaticCodeBlock>
    );
  },
  ul: ({ children, ...props }) => (
    <ul className="list-inside list-disc" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }) => (
    <ol className="list-inside list-decimal" {...props}>
      {children}
    </ol>
  ),
};

// This file is required to use MDX in `app` directory.
export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...components,
    ...CustomMDXComponents,
  };
}
