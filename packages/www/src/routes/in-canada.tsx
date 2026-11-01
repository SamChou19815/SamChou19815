import { createFileRoute } from "@tanstack/react-router";
import { appIconHead } from "../lib/appIcons";
import InCanadaApp from "../app/in-canada/InCanadaApp";
import AuthGate, { signedInEmail } from "../lib/AuthGate";
import { useOwnerAllowlist } from "../lib/useOwnerAllowlist";
import { useAuth } from "../lib/useAuth";

export const Route = createFileRoute("/in-canada")({
  head: () => {
    const icon = appIconHead("in-canada", "In-Canada");
    return {
      meta: [
        { title: "In-Canada Days Counter | Developer Sam" },
        { name: "robots", content: "noindex, nofollow" },
        ...icon.meta,
      ],
      links: icon.links,
    };
  },
  component: CanadaPage,
});

function CanadaPage(): React.JSX.Element {
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
