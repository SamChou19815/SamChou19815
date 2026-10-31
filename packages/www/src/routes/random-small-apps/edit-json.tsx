import { createFileRoute } from "@tanstack/react-router";
import JsonEditorApp from "../../app/random-small-apps/edit-json/JsonEditorApp";
import AuthGate, { signedInEmail } from "../../lib/AuthGate";
import { useOwnerAllowlist } from "../../lib/useOwnerAllowlist";
import { useAuth } from "../../lib/useAuth";

export const Route = createFileRoute("/random-small-apps/edit-json")({
  head: () => ({
    meta: [
      { title: "Edit JSON | Developer Sam" },
      { name: "robots", content: "noindex, nofollow" },
    ],
  }),
  component: EditJsonPage,
});

function EditJsonPage(): React.JSX.Element {
  const auth = useAuth();
  const access = useOwnerAllowlist(signedInEmail(auth));
  return (
    <AuthGate
      title="JSON Editor"
      signedOutPrompt="Sign in to edit app data."
      deniedMessage="This app is restricted to its owner."
      auth={auth}
      access={access}
    >
      <JsonEditorApp />
    </AuthGate>
  );
}
