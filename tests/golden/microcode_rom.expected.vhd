library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_microcode_rom is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_micro_pc : in unsigned(7 downto 0);
    gl_p2_instruction : out unsigned(23 downto 0)
  );
end entity gl_m0_microcode_rom;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_microcode_rom is
  type gl_rom_microcode_t is array (
    0 to 255
) of unsigned(23 downto 0);
  constant gl_rom_microcode : gl_rom_microcode_t := (
    0 => "000000000000000000000000",
    1 => "000100000000000000000001",
    2 => "001000000000000000000010",
    10 => "111111111111111111111111",
    others => "000000000000000000000000"
  );
  signal gl_s2_instruction : unsigned(23 downto 0);
  signal gl_s3_instruction_reg : unsigned(23 downto 0) := resize(unsigned'(x"0000000000000000"), 24);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_instruction <= gl_s3_instruction_reg;
  end process gl_comb_0;
  gl_seq_0 : process(gl_p0_clk)
  begin
    if rising_edge(gl_p0_clk) then
      gl_s3_instruction_reg <= gl_rom_microcode(to_integer(gl_p1_micro_pc));
    end if;
  end process gl_seq_0;
  gl_p2_instruction <= gl_s2_instruction;
end architecture rtl;
