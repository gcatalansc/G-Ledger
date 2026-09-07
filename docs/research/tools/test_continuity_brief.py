"""Research prototype checks: information boundaries, not user efficacy."""
import copy
import unittest

from continuity_brief import build_brief


def fixture():
    return {"schema_version": "0.2.0", "project_ref": "project:fixture", "state_version": 19,
            "state_digest": "sha256:" + "a" * 64, "checkpoint_freshness": "INVALIDATED",
            "active_phase_refs": [], "completed_phase_refs": [], "current_decision_refs": [],
            "open_blocker_refs": [], "open_risk_refs": [], "pending_gate_refs": [],
            "critical_unknown_refs": [f"unknown:dependency-{i}" for i in range(8)],
            "conflict_refs": [], "preferred_next_action_ref": None,
            "last_material_progress_summary": "Regresión focalizada completada…"}


class ContinuityBriefTests(unittest.TestCase):
    def test_all_eight_unknowns_are_counted_without_claiming_four_are_complete(self):
        self.assertIn("Incógnitas críticas: 8", build_brief(fixture(), "fixture.json"))

    def test_truncated_progress_is_explicitly_incomplete(self):
        self.assertIn("Texto recortado", build_brief(fixture(), "fixture.json"))

    def test_missing_next_action_does_not_become_authorization(self):
        self.assertIn("sin registro; no establece autorización", build_brief(fixture(), "fixture.json"))

    def test_an_existing_reference_does_not_imply_permission(self):
        value = fixture()
        value["preferred_next_action_ref"] = "action:proposed"
        self.assertIn("clasificación y precondiciones requieren consulta", build_brief(value, "fixture.json"))

    def test_project_only_freshness_does_not_claim_repository_freshness(self):
        value = fixture()
        value["checkpoint_freshness"] = "CURRENT_FOR_PROJECT_ONLY"
        self.assertIn("no verifica el checkout externo", build_brief(value, "fixture.json"))

    def test_invalid_shape_is_rejected_and_source_is_not_mutated(self):
        value = fixture()
        saved = copy.deepcopy(value)
        build_brief(value, "fixture.json")
        self.assertEqual(saved, value)
        value["critical_unknown_refs"] = "eight"
        with self.assertRaises(ValueError):
            build_brief(value, "fixture.json")


if __name__ == "__main__":
    unittest.main()
