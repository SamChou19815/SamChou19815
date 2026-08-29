"use client";

import AuthGate from "../../lib/AuthGate";
import InCanadaApp from "./InCanadaApp";
import { useOwnerAllowlist } from "./useOwnerAllowlist";

export default function CanadaPage(): React.JSX.Element {
  return (
    <AuthGate
      title="In-Canada"
      signedOutPrompt="Sign in to view the counter."
      deniedMessage="This app is restricted to its owner."
      useAccessCheck={useOwnerAllowlist}
    >
      <InCanadaApp />
    </AuthGate>
  );
}
