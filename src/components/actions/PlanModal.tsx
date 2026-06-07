import { useStore } from "../../store/appStore";
import PlanView from "./PlanView";
import Pebble from "../Pebble";

export default function PlanModal() {
  const plan = useStore((s) => s.activePlan);
  if (!plan) return null;
  return (
    <div className="overlay">
      <div className="modal">
        <h3>
          <Pebble size={26} className="peb" /> Pebble has a little plan
        </h3>
        <PlanView plan={plan} />
      </div>
    </div>
  );
}
