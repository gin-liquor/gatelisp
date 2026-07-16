library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_generic_case is
  generic (
    gl_g0 : positive := 8
  );
  port (
    gl_p0_selector : in std_logic;
    gl_p1_a : in unsigned(gl_g0 - 1 downto 0);
    gl_p2_b : in unsigned(gl_g0 - 1 downto 0);
    gl_p3_value : out unsigned(gl_g0 - 1 downto 0)
  );
end entity gl_m0_generic_case;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_generic_case is
  signal gl_s3_value : unsigned(gl_g0 - 1 downto 0);
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
    variable gl_tmp_0 : std_logic;
    variable gl_tmp_1 : unsigned(gl_g0 - 1 downto 0);
  begin
    gl_tmp_0 := gl_p0_selector;
    if (gl_tmp_0 = '0') then
      gl_tmp_1 := gl_p1_a;
    else
      if (gl_tmp_0 = '1') then
        gl_tmp_1 := gl_p2_b;
      else
        gl_tmp_1 := gl_p1_a;
      end if;
    end if;
    gl_s3_value <= gl_tmp_1;
  end process gl_comb_0;
  gl_p3_value <= gl_s3_value;
end architecture rtl;
