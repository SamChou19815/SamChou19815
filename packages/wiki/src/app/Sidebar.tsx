import React, { ReactElement } from 'react';

import DocSidebar from '@theme/DocSidebar';

import type { WikiPrivateDocumentMetadata } from './documents';

type Props = {
  readonly className?: string;
  readonly selectedDocumentID: string | null;
  readonly documentMetadataList: readonly WikiPrivateDocumentMetadata[];
};

type SideBarEntry =
  | { readonly type: 'link'; readonly label: string; readonly href: string }
  | {
      readonly type: 'category';
      readonly label: string;
      readonly collapsed: true;
      readonly items: SideBarEntry[];
    };

const treeifySingleDocumentMedatada = ({
  documentID,
  filename,
}: WikiPrivateDocumentMetadata): SideBarEntry => {
  const filenameSegments = filename.split('/');
  const lastSegment = filenameSegments[filenameSegments.length - 1];
  if (lastSegment == null) throw new Error();
  let entry: SideBarEntry = {
    type: 'link',
    label: lastSegment,
    href: `/intern#doc-${documentID}`,
  };
  for (let i = filenameSegments.length - 2; i >= 0; i -= 1) {
    const label = filenameSegments[i];
    if (label == null) throw new Error();
    entry = { type: 'category', label, collapsed: true, items: [entry] };
  }
  return entry;
};

const mergeTrees = (existingTrees: SideBarEntry[], entry: SideBarEntry): void => {
  if (entry.type === 'link') {
    existingTrees.push(entry);
    return;
  }
  for (let i = 0; i < existingTrees.length; i += 1) {
    const mergeCandidate = existingTrees[i];
    if (mergeCandidate == null) throw new Error();
    if (mergeCandidate.type === 'category' && mergeCandidate.label === entry.label) {
      const firstItem = entry.items[0];
      if (firstItem == null) throw new Error();
      mergeTrees(mergeCandidate.items, firstItem);
      return;
    }
  }
  existingTrees.push(entry);
};

const treeifyDocumentMetadata = (
  documentMetadataList: readonly WikiPrivateDocumentMetadata[]
): readonly SideBarEntry[] => {
  const mergedEntries: SideBarEntry[] = [];
  documentMetadataList
    .map(treeifySingleDocumentMedatada)
    .forEach((entry) => mergeTrees(mergedEntries, entry));
  return mergedEntries;
};

const Sidebar = ({ className, selectedDocumentID, documentMetadataList }: Props): ReactElement => {
  return (
    <div className={className} role="complementary">
      <DocSidebar
        sidebar={treeifyDocumentMetadata(documentMetadataList)}
        path={`/intern${selectedDocumentID == null ? '' : `#doc-${selectedDocumentID}`}`}
        sidebarCollapsible
        isHidden={false}
        onCollapse={() => {}}
      />
    </div>
  );
};

export default Sidebar;
