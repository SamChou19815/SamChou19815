import React, { ReactElement, useState, useEffect } from 'react';

import DocSidebar from '@theme/DocSidebar';
import clsx from 'clsx';

import styles from './App.module.css';
import MarkdownBlock from './MarkdownBlock';
import { getAppUser, isAdminUser } from './authentication';
import { useWikiPrivateDocuments } from './documents';
import MarkdownEditorWithPreview from './MarkdownEditorWithPreview';

const App = (): ReactElement => {
  const [documentID, setDocumentID] = useState<string | null>(null);

  useEffect(() => {
    const handle = setInterval(() => {
      setDocumentID(location.hash.startsWith('#doc-') ? location.hash.substring(5) : null);
    }, 50);
    () => clearInterval(handle);
  });

  const documents = useWikiPrivateDocuments();

  if (documents == null) {
    return <div className="simple-page-center">Loading...</div>
  }

  const documentToRender = documents.find(document => document.documentID === documentID);
  const markdownCode = documentToRender != null
    ? `# ${documentToRender.title}\n\n${documentToRender.markdownContent}`
    : `# Hello ${getAppUser().displayName}!\nSelect a document on the left`;

  return (
    <div className={styles.App}>
      <div className={styles.DocumentSidebarContainer} role="complementary">
        <DocSidebar
          sidebar={[
            {
              label: 'Documents Shared with You',
              type: 'category',
              collapsed: false,
              items: documents.map(({ documentID: id, title }) => ({
                type: 'link', label: title, href: `/intern#doc-${id}`
              }))
            }
          ]}
          path={`/intern${documentID == null ? '' : `#doc-${documentID}`}`}
          sidebarCollapsible
        />
      </div>
      <main className={clsx('container', styles.DocumentMainContainer)}>
        <MarkdownBlock markdownCode={markdownCode} />
        {documentToRender != null && isAdminUser() && (
          <MarkdownEditorWithPreview initialMarkdownCode={documentToRender.markdownContent} />
        )}
      </main>
    </div>
  );
}

export default App;
