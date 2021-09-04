export type BlogPaginationItem = { readonly title: string; readonly permalink: string };

export type Metadata = {
  readonly date: string;
  readonly formattedDate: string;
  readonly permalink: string;
  readonly nextItem?: BlogPaginationItem;
  readonly prevItem?: BlogPaginationItem;
};