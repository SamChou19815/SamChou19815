import Link from 'esbuild-scripts/components/Link';
import MDXProvider from 'esbuild-scripts/components/MDXProvider';
import React from 'react';
import Article from './Article';
import MDXComponents from './MDXComponents';

type Props = {
  readonly metadata: BlogPostMetadata;
  readonly truncated?: boolean;
  readonly children?: JSX.Element;
};

export default function BlogPostItem(props: Props): JSX.Element {
  const { children, metadata, truncated } = props;
  const { title, formattedDate, permalink } = metadata;

  const TitleHeading = truncated ? 'h2' : 'h1';

  return (
    <Article>
      <header>
        <TitleHeading className="mb-2 font-sans">
          {truncated ? <Link to={permalink}>{title}</Link> : title}
        </TitleHeading>
        <div className="my-4">{formattedDate}</div>
      </header>
      {children && (
        <div className="markdown">
          <MDXProvider components={MDXComponents}>{children}</MDXProvider>
        </div>
      )}
    </Article>
  );
}
