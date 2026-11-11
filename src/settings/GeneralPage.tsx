import { useClock24 } from "@/lib/clock";
import { COUNTRIES, useCountry, WORLDWIDE } from "@/lib/country";
import { ChoiceRow, SwitchRow } from "./controls";
import { Section } from "./rows";

export default function GeneralPage() {
  const [country, setCountry] = useCountry();
  const [clock24, setClock24] = useClock24();

  return (
    <Section title="General" cloud="general">
      <ChoiceRow
        title="Country"
        summary="Charts and recommendations follow it"
        // nothing chosen reads as worldwide, which is what the charts use then
        value={country || WORLDWIDE}
        options={COUNTRIES}
        onChange={setCountry}
      />
      <SwitchRow title="24-hour time" summary="Shows 13:05 instead of 1:05 PM" checked={clock24} onChange={setClock24} />
    </Section>
  );
}
