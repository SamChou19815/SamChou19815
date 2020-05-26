import React, { ReactElement, ReactNode } from 'react';

import classnames from 'classnames';

type Props = {
  readonly href: string;
  readonly className?: string;
  readonly children: ReactNode;
};

const ButtonLink = ({ href, children, className }: Props): ReactElement => (
  <a className={classnames('button', 'button--link', className)} href={href}>
    {typeof children === 'string' ? children.toLocaleUpperCase() : children}
  </a>
);

export default ButtonLink;
