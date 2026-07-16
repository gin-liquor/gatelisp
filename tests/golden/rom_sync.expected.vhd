library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_rom_sync is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_address : in unsigned(2 downto 0);
    gl_p2_value : out unsigned(7 downto 0)
  );
end entity gl_m0_rom_sync;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_rom_sync is
  type gl_rom_table_t is array (
    0 to 7
) of unsigned(7 downto 0);
  constant gl_rom_table : gl_rom_table_t := (
    0 => "00000001",
    2 => "00000100",
    7 => "11111111",
    others => "00000000"
  );
  signal gl_s2_value : unsigned(7 downto 0);
  signal gl_s3_output : unsigned(7 downto 0) := resize(unsigned'(x"0000000000000000"), 8);
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
    gl_s2_value <= gl_s3_output;
  end process gl_comb_0;
  gl_seq_0 : process(gl_p0_clk)
  begin
    if rising_edge(gl_p0_clk) then
      gl_s3_output <= gl_rom_table(to_integer(gl_p1_address));
    end if;
  end process gl_seq_0;
  gl_p2_value <= gl_s2_value;
end architecture rtl;
