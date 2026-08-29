export type BlogPostMetadata = {
  readonly title: string;
  readonly year: string;
  readonly month: string;
  readonly date: string;
  readonly titleSlug: string;
  /** The post's hero image, from its frontmatter. Used as the OpenGraph card. */
  readonly image?: string;
  /**
   * When set, the post is hosted elsewhere. It is listed in the blog index but links out to this
   * URL instead of opening in the terminal's reader.
   */
  readonly externalUrl?: string;
};

export const allMetadata: readonly BlogPostMetadata[] = JSON.parse(
  process.env.ALL_BLOG_POST_METADATA as string,
);

export function permalinkFromMetadata(metadata: BlogPostMetadata): string {
  if (metadata.externalUrl != null) return metadata.externalUrl;
  return `/blog/${metadata.year}/${metadata.month}/${metadata.date}/${metadata.titleSlug}`;
}

export function isExternalPost(metadata: BlogPostMetadata): boolean {
  return metadata.externalUrl != null;
}
