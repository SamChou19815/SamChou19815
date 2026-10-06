"use client";

import AuthGate, { signedInEmail } from "../../lib/AuthGate";
import { useAuth } from "../../lib/useAuth";
import InCanadaApp from "./InCanadaApp";
import { useOwnerAllowlist } from "./useOwnerAllowlist";

export default function CanadaPage(): React.JSX.Element {
  const auth = useAuth();
  const access = useOwnerAllowlist(signedInEmail(auth));
  return (
    <AuthGate
      title="In-Canada"
      signedOutPrompt="Sign in to view the counter."
      deniedMessage="This app is restricted to its owner."
      auth={auth}
      access={access}
    >
      <InCanadaApp />
    </AuthGate>
  );
}
