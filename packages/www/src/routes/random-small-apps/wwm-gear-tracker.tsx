import { createFileRoute } from "@tanstack/react-router";
import { appIconHead } from "../../lib/appIcons";
import WwmGearTrackerApp from "../../app/random-small-apps/wwm-gear-tracker/WwmGearTrackerApp";
import AuthGate, { signedInEmail } from "../../lib/AuthGate";
import { useOwnerAllowlist } from "../../lib/useOwnerAllowlist";
import { useAuth } from "../../lib/useAuth";

export const Route = createFileRoute("/random-small-apps/wwm-gear-tracker")({
  head: () => {
    const icon = appIconHead("wwm-gear-tracker", "装备词条");
    return {
      meta: [
        { title: "燕云十六声 · 装备词条 | Developer Sam" },
        { name: "robots", content: "noindex, nofollow" },
        ...icon.meta,
      ],
      links: icon.links,
    };
  },
  component: WwmGearTrackerPage,
});

function WwmGearTrackerPage(): React.JSX.Element {
  const auth = useAuth();
  const access = useOwnerAllowlist(signedInEmail(auth));
  return (
    <AuthGate
      title="燕云十六声 · 装备词条"
      signedOutPrompt="Sign in to track your gear."
      deniedMessage="This app is restricted to its owner."
      auth={auth}
      access={access}
    >
      <WwmGearTrackerApp />
    </AuthGate>
  );
}
