import { GameModes } from "@/components/GameModes";
import { puzzle } from "@/lib/puzzle";

export default function Home() {
  return <GameModes weekly={puzzle} />;
}
