import React, { ReactElement, AnchorHTMLAttributes } from 'react';
import { Link as RouterLink } from 'react-router-dom';

type Props = AnchorHTMLAttributes<HTMLAnchorElement> & { readonly to: string };

const Link = ({ to, children, ...props }: Props): ReactElement => {
  if (__SERVER__) {
    return <a {...{ ...props, href: to }}>{children}</a>;
  }
  return <RouterLink {...{ ...props, to }}>{children}</RouterLink>;
};

export default Link;