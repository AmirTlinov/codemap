package api

import "testing"

func TestContractVersion(t *testing.T) {
	if ContractVersion() != 1 {
		t.Fatal("bad contract")
	}
}
